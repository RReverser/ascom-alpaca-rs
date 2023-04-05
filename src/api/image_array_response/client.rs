use super::{
    ImageArrayResponse, ImageArrayResponseRank, ImageBytesMetadata, COLOUR_AXIS, IMAGE_BYTES_TYPE,
};
use crate::api::ImageArrayResponseType;
use crate::client::{Response, ResponseTransaction, ResponseWithTransaction};
use crate::{ASCOMError, ASCOMErrorCode, ASCOMResult};
use bytes::Bytes;
use mime::Mime;
use ndarray::{Array2, Array3};
use serde::de::{DeserializeOwned, IgnoredAny, Visitor};
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq, Eq, Clone, Copy)]
#[serde(field_identifier)]
enum KnownKey {
    Type,
    Rank,
    Value,
    #[serde(other)]
    Other,
}

fn expect_key<'de, A: serde::de::MapAccess<'de>>(
    map: &mut A,
    expected_key: KnownKey,
) -> Result<(), A::Error> {
    loop {
        return match map.next_key::<KnownKey>()? {
            Some(KnownKey::Other) => {
                let _ = map.next_value::<IgnoredAny>()?;
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
struct ResponseData<A>(#[serde(with = "serde_ndim")] A)
where
    A: serde_ndim::de::MakeNDim,
    A::Item: DeserializeOwned;

struct ResponseVisitor;

impl<'de> Visitor<'de> for ResponseVisitor {
    type Value = ImageArrayResponse;

    fn expecting(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.write_str("a map")
    }

    fn visit_map<A: serde::de::MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        expect_key(&mut map, KnownKey::Type)?;
        let type_ = map.next_value::<ImageArrayResponseType>()?;
        if type_ != ImageArrayResponseType::Integer {
            return Err(serde::de::Error::custom(format!(
                r"expected Type == Integer, got {type_:?}",
            )));
        }

        expect_key(&mut map, KnownKey::Rank)?;
        let rank = map.next_value::<ImageArrayResponseRank>()?;

        expect_key(&mut map, KnownKey::Value)?;
        let data = match rank {
            ImageArrayResponseRank::Rank2 => map
                .next_value::<ResponseData<Array2<i32>>>()?
                .0
                .insert_axis(COLOUR_AXIS),
            ImageArrayResponseRank::Rank3 => map.next_value::<ResponseData<Array3<i32>>>()?.0,
        };

        // Consume leftover fields.
        let _ = IgnoredAny.visit_map(map)?;

        Ok(ImageArrayResponse { data })
    }
}

struct JsonImageArrayResponse(ImageArrayResponse);

impl<'de> Deserialize<'de> for JsonImageArrayResponse {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_map(ResponseVisitor).map(Self)
    }
}

impl Response for ASCOMResult<ImageArrayResponse> {
    fn prepare_reqwest(request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        request.header(reqwest::header::ACCEPT, IMAGE_BYTES_TYPE)
    }

    fn from_reqwest(
        mime_type: Mime,
        bytes: Bytes,
    ) -> anyhow::Result<ResponseWithTransaction<Self>> {
        if mime_type.essence_str() != IMAGE_BYTES_TYPE {
            return <ASCOMResult<JsonImageArrayResponse>>::from_reqwest(mime_type, bytes)
                .map(|response| response.map(|response| response.map(|json| json.0)));
        }
        let metadata = bytes
            .get(..std::mem::size_of::<ImageBytesMetadata>())
            .ok_or_else(|| anyhow::anyhow!("not enough bytes to read image metadata"))?;
        let metadata = bytemuck::pod_read_unaligned::<ImageBytesMetadata>(metadata);
        anyhow::ensure!(
            metadata.metadata_version == 1_i32,
            "unsupported metadata version {}",
            metadata.metadata_version
        );
        let data_start = usize::try_from(metadata.data_start)?;
        anyhow::ensure!(
            data_start >= std::mem::size_of::<ImageBytesMetadata>(),
            "image data start offset is within metadata"
        );
        let data = bytes
            .get(data_start..)
            .ok_or_else(|| anyhow::anyhow!("image data start offset is out of bounds"))?;
        let transaction = ResponseTransaction {
            client_transaction_id: metadata.client_transaction_id,
            server_transaction_id: metadata.server_transaction_id,
        };
        let ascom_result = if metadata.error_number == 0_i32 {
            anyhow::ensure!(
                metadata.image_element_type == ImageArrayResponseType::Integer as i32,
                "only Integer image element type is supported, got {}",
                metadata.image_element_type
            );
            let data = match metadata.transmission_element_type {
				1_i32 /* Int16 */=> bytemuck::cast_slice::<u8, i16>(data)
					.iter()
					.copied()
					.map(i32::from)
					.collect(),
				2_i32/* Int32 */ => bytemuck::cast_slice::<u8, i32>(data).to_owned(),
				6_i32/* Byte */ => data.iter().copied().map(i32::from).collect(),
				8_i32 /* Uint16 */=> bytemuck::cast_slice::<u8, u16>(data)
					.iter()
					.copied()
					.map(i32::from)
					.collect(),
				_ => anyhow::bail!(
					"unsupported integer transmission type {}",
					metadata.transmission_element_type
				),
			};
            let shape = ndarray::Ix3(
                usize::try_from(metadata.dimension_1)?,
                usize::try_from(metadata.dimension_2)?,
                match metadata.rank {
                    2 => {
                        anyhow::ensure!(
                            metadata.dimension_3 == 0_i32,
                            "dimension 3 must be 0 for rank 2, got {}",
                            metadata.dimension_3
                        );
                        1
                    }
                    3 => usize::try_from(metadata.dimension_3)?,
                    rank => anyhow::bail!("unsupported rank {}, expected 2 or 3", rank),
                },
            );
            Ok(ImageArrayResponse {
                data: ndarray::Array::from_shape_vec(shape, data)
                    .expect("couldn't match the parsed shape to the data"),
            })
        } else {
            Err(ASCOMError::new(
                ASCOMErrorCode::try_from(u16::try_from(metadata.error_number)?)?,
                std::str::from_utf8(data)?.to_owned(),
            ))
        };
        Ok(ResponseWithTransaction {
            transaction,
            response: ascom_result,
        })
    }
}
