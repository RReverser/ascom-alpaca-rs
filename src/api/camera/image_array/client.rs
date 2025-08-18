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
            mismatched_key_or_none => Err(serde::de::Error::custom(format!(
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
    Ok(bytemuck::try_cast_slice::<u8, T>(data)?
        .iter()
        .copied()
        .map(T::into)
        .collect())
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
        let metadata = bytes
            .get(..size_of::<ImageBytesMetadata>())
            .ok_or_else(|| eyre::eyre!("not enough bytes to read image metadata"))?;
        let metadata = bytemuck::try_from_bytes::<ImageBytesMetadata>(metadata)?;
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
