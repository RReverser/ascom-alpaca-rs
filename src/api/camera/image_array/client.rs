use super::{
    AsTransmissionElementType, ImageArray, ImageArrayRank, ImageBytesMetadata, ImageElementType,
    TransmissionElementType, COLOUR_AXIS, IMAGE_BYTES_TYPE,
};
use crate::client::{Response, ResponseTransaction, ResponseWithTransaction};
use crate::{ASCOMError, ASCOMErrorCode, ASCOMResult};
use bytemuck::PodCastError;
use mime::Mime;
use ndarray::{Array2, Array3};
use num_enum::TryFromPrimitive;
use serde::de::{DeserializeOwned, IgnoredAny, MapAccess, Visitor};
use serde::Deserialize;
use serde_ndim::de::MakeNDim;
use std::fmt::{self, Formatter};

#[derive(Debug, Deserialize, PartialEq, Eq, Clone, Copy)]
#[serde(field_identifier)]
enum KnownKey {
    Type,
    Rank,
    Value,
    #[serde(other)]
    Other,
}

fn expect_key<'de, A: MapAccess<'de>>(map: &mut A, expected_key: KnownKey) -> Result<(), A::Error> {
    loop {
        return match map.next_key::<KnownKey>()? {
            Some(KnownKey::Other) => {
                _ = map.next_value::<IgnoredAny>()?;
                continue;
            }
            Some(key) if key == expected_key => Ok(()),
            mismatched_key_or_none => Err(serde::de::Error::custom(format_args!(
                "expected field {expected_key:?}, got {mismatched_key_or_none:?}"
            ))),
        };
    }
}

#[derive(Deserialize)]
#[serde(transparent)]
struct ResponseData<A: MakeNDim<Item: DeserializeOwned>>(#[serde(with = "serde_ndim")] A);

struct ResponseVisitor;

impl<'de> Visitor<'de> for ResponseVisitor {
    type Value = ImageArray;

    fn expecting(&self, fmt: &mut Formatter<'_>) -> fmt::Result {
        fmt.write_str("a map")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        expect_key(&mut map, KnownKey::Type)?;
        let ImageElementType::I32 = map.next_value::<ImageElementType>()?;

        expect_key(&mut map, KnownKey::Rank)?;
        let rank = map.next_value::<ImageArrayRank>()?;

        expect_key(&mut map, KnownKey::Value)?;
        let data = match rank {
            ImageArrayRank::Rank2 => map
                .next_value::<ResponseData<Array2<i32>>>()?
                .0
                .insert_axis(COLOUR_AXIS),
            ImageArrayRank::Rank3 => map.next_value::<ResponseData<Array3<i32>>>()?.0,
        };

        // Consume leftover fields.
        _ = IgnoredAny.visit_map(map)?;

        Ok(data.into())
    }
}

struct JsonImageArray(ImageArray);

impl<'de> Deserialize<'de> for JsonImageArray {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_map(ResponseVisitor).map(Self)
    }
}

fn cast_raw_data<T: AsTransmissionElementType>(data: &[u8]) -> Result<Vec<i32>, PodCastError> {
    // Fast path: if `data` is already aligned to `align_of::<T>()`,
    // `try_cast_slice` reinterprets in place and we iterate-and-widen
    // into the output `Vec<i32>`. This is the common case — most host
    // allocators hand out 16-aligned buffers so the body slice that
    // reqwest hands us satisfies a 4-byte alignment requirement.
    //
    // Slow path: the HTTP response body is a `bytes::Bytes` slice
    // from reqwest's chunked read, and its start pointer is not
    // *guaranteed* to be `align_of::<T>()`-aligned (4 bytes for
    // `i32`, 2 for `i16` / `u16`). When the fast cast fails with
    // `TargetAlignmentGreaterAndInputNotAligned`, hand the bytes to
    // `T::widen_unaligned`, which splits `data` into
    // `size_of::<T>()`-wide chunks and reads each one with
    // `bytemuck::pod_read_unaligned` straight into the output
    // `Vec<i32>` — one pass, no intermediate aligned `Vec<T>`. A
    // non-empty `chunks_exact` remainder reproduces the
    // `OutputSliceWouldHaveSlop` guarantee. Other cast errors
    // propagate verbatim — only the alignment failure falls back.
    match bytemuck::try_cast_slice::<u8, T>(data) {
        Ok(aligned) => Ok(aligned.iter().copied().map(T::into).collect()),
        Err(PodCastError::TargetAlignmentGreaterAndInputNotAligned) => T::widen_unaligned(data),
        Err(other) => Err(other),
    }
}

impl Response for ASCOMResult<ImageArray> {
    fn prepare_reqwest(request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        request.header(reqwest::header::ACCEPT, IMAGE_BYTES_TYPE)
    }

    fn from_reqwest(mime_type: Mime, bytes: &[u8]) -> eyre::Result<ResponseWithTransaction<Self>> {
        if mime_type.essence_str() != IMAGE_BYTES_TYPE {
            let transaction = ResponseTransaction::from_reqwest(mime_type, bytes)?;
            let ascom_error = serde_json::from_slice::<ASCOMError>(bytes)?;

            return Ok(ResponseWithTransaction {
                transaction,
                response: match ascom_error.code {
                    ASCOMErrorCode::OK => Ok(serde_json::from_slice::<JsonImageArray>(bytes)?.0),
                    _ => Err(ascom_error),
                },
            });
        }
        let metadata_bytes = bytes
            .get(..size_of::<ImageBytesMetadata>())
            .ok_or_else(|| eyre::eyre!("not enough bytes to read image metadata"))?;
        // `bytemuck::try_from_bytes::<ImageBytesMetadata>` requires
        // the source slice to be aligned to
        // `align_of::<ImageBytesMetadata>()`. The body buffer's start
        // pointer is not guaranteed to satisfy that (see
        // `cast_raw_data` for the parallel issue);
        // `pod_read_unaligned` reads via `ptr::read_unaligned` and
        // works for any pointer.
        let metadata: ImageBytesMetadata = bytemuck::pod_read_unaligned(metadata_bytes);
        eyre::ensure!(
            metadata.metadata_version == 1_i32,
            "unsupported metadata version {}",
            metadata.metadata_version,
        );
        let data_start = usize::try_from(metadata.data_start)?;
        eyre::ensure!(
            data_start >= size_of::<ImageBytesMetadata>(),
            "image data start offset is within metadata",
        );
        let raw_data = bytes
            .get(data_start..)
            .ok_or_else(|| eyre::eyre!("image data start offset is out of bounds"))?;
        let transaction = ResponseTransaction {
            client_transaction_id: metadata.client_transaction_id,
            server_transaction_id: metadata.server_transaction_id,
        };
        let ascom_result = if metadata.error_number == 0_i32 {
            let ImageElementType::I32 =
                ImageElementType::try_from_primitive(metadata.image_element_type)?;
            let transmission_element_type =
                TransmissionElementType::try_from_primitive(metadata.transmission_element_type)?;
            let data = match transmission_element_type {
                TransmissionElementType::I16 => cast_raw_data::<i16>(raw_data),
                TransmissionElementType::I32 => cast_raw_data::<i32>(raw_data),
                TransmissionElementType::U8 => cast_raw_data::<u8>(raw_data),
                TransmissionElementType::U16 => cast_raw_data::<u16>(raw_data),
            }?;
            let shape = ndarray::Ix3(
                usize::try_from(metadata.dimension_1)?,
                usize::try_from(metadata.dimension_2)?,
                match ImageArrayRank::try_from_primitive(metadata.rank)? {
                    ImageArrayRank::Rank2 => {
                        eyre::ensure!(
                            metadata.dimension_3 == 0_i32,
                            "dimension 3 must be 0 for rank 2, got {}",
                            metadata.dimension_3,
                        );
                        1
                    }
                    ImageArrayRank::Rank3 => usize::try_from(metadata.dimension_3)?,
                },
            );
            Ok(ndarray::Array::from_shape_vec(shape, data)?.into())
        } else {
            Err(ASCOMError::new(
                ASCOMErrorCode::try_from(u16::try_from(metadata.error_number)?)?,
                std::str::from_utf8(raw_data)?.to_owned(),
            ))
        };
        Ok(ResponseWithTransaction {
            transaction,
            response: ascom_result,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build an `application/imagebytes` payload at the given byte
    /// offset within a `Vec<u8>`. The `Vec`'s base allocation is
    /// usually a high-alignment chunk from the system allocator, so
    /// looping `leading_pad` over `0..align_of::<i32>()` is sufficient
    /// to cover all four `mod 4` cases — at least three of those
    /// rotations land the body slice on a non-4-aligned start, which
    /// is what reproduces the upstream-pre-fix
    /// `TargetAlignmentGreaterAndInputNotAligned` path.
    fn imagebytes_payload_at_offset(
        leading_pad: usize,
        pixels: &[i32],
    ) -> eyre::Result<(Vec<u8>, std::ops::Range<usize>)> {
        let metadata = ImageBytesMetadata {
            metadata_version: 1_i32,
            error_number: 0_i32,
            client_transaction_id: None,
            server_transaction_id: None,
            data_start: i32::try_from(size_of::<ImageBytesMetadata>())?,
            image_element_type: i32::from(TransmissionElementType::I32),
            transmission_element_type: i32::from(TransmissionElementType::I32),
            rank: i32::from(ImageArrayRank::Rank2),
            dimension_1: i32::try_from(pixels.len())?,
            dimension_2: 1_i32,
            dimension_3: 0_i32,
        };
        let mut buf = vec![0u8; leading_pad];
        let payload_start = buf.len();
        buf.extend_from_slice(bytemuck::bytes_of(&metadata));
        buf.extend_from_slice(bytemuck::cast_slice(pixels));
        let payload_end = buf.len();
        Ok((buf, payload_start..payload_end))
    }

    /// Regression test for issue #18: parsing an `imagebytes` body
    /// whose slice start is not 4-byte aligned must succeed.
    /// Pre-fix this path failed with
    /// `bytemuck::PodCastError::TargetAlignmentGreaterAndInputNotAligned`
    /// because both the metadata `try_from_bytes` and the pixel
    /// `try_cast_slice` require source alignment. After the fix —
    /// metadata via `pod_read_unaligned`, and pixels via an aligned
    /// `try_cast_slice` fast path with a `pod_read_unaligned`
    /// fallback — the parse is alignment-independent.
    fn check_one_offset(leading_pad: usize, pixels: &[i32]) -> eyre::Result<()> {
        let (buf, range) = imagebytes_payload_at_offset(leading_pad, pixels)?;
        let slice = &buf[range];
        let mime: Mime = IMAGE_BYTES_TYPE.parse()?;
        let parsed = <ASCOMResult<ImageArray> as Response>::from_reqwest(mime, slice)?;
        let array = parsed
            .response
            .map_err(|e| eyre::eyre!("ascom error: {e:?}"))?;
        eyre::ensure!(
            array.shape() == [pixels.len(), 1_usize, 1_usize],
            "shape mismatch: {:?}",
            array.shape()
        );
        for (i, &p) in pixels.iter().enumerate() {
            let actual = array[[i, 0_usize, 0_usize]];
            eyre::ensure!(actual == p, "pixel {i} mismatch: got {actual}, expected {p}");
        }
        Ok(())
    }

    #[test]
    fn from_reqwest_handles_unaligned_imagebytes_slice() {
        let pixels: Vec<i32> = (0_i32..16_i32).collect();
        // Try every offset 0..4 — at least three of these rotations
        // land the body slice on a non-4-aligned start (regardless of
        // the `Vec<u8>` base alignment), which is the case that fails
        // pre-fix.
        for leading_pad in 0_usize..4_usize {
            check_one_offset(leading_pad, &pixels)
                .unwrap_or_else(|e| panic!("leading_pad={leading_pad}: {e:?}"));
        }
    }

    /// The shared `widen_unaligned` default method must widen each
    /// element type the way the aligned fast path does: sign-extend
    /// signed types, zero-extend unsigned ones, pass `i32` through. The
    /// `#18` regression test above only drives the `i32` width, so this
    /// locks in the `size_of::<Self>()` dispatch and the sign/zero
    /// extension for the remaining widths.
    #[test]
    fn widen_unaligned_widens_each_element_type() {
        assert_eq!(
            i16::widen_unaligned(&[0x00_u8, 0x80, 0xff, 0x7f]).expect("even length"),
            vec![i32::from(i16::MIN), i32::from(i16::MAX)],
        );
        assert_eq!(
            u16::widen_unaligned(&[0x00_u8, 0x80, 0xff, 0xff]).expect("even length"),
            vec![0x8000_i32, 0xffff_i32],
        );
        assert_eq!(
            u8::widen_unaligned(&[0x00_u8, 0x80, 0xff]).expect("any length"),
            vec![0_i32, 128_i32, 255_i32],
        );
        assert_eq!(
            i32::widen_unaligned(&[0xff_u8, 0xff, 0xff, 0xff]).expect("multiple of four"),
            vec![-1_i32],
        );
        // A trailing partial element reproduces the fast path's
        // `OutputSliceWouldHaveSlop` rejection.
        assert!(i16::widen_unaligned(&[0x00_u8, 0x80, 0x01]).is_err());
    }
}
