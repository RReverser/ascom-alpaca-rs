use crate::api::ImageArrayResponseType;
use crate::ASCOMResult;
use bytemuck::{Pod, Zeroable};
use ndarray::{Array2, Array3, Axis};
use serde::de::{DeserializeOwned, IgnoredAny, Visitor};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::num::NonZeroU32;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum ImageArrayResponseRank {
    Rank2 = 2,
    Rank3 = 3,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ImageArrayResponse {
    pub data: Array3<i32>,
}

const COLOUR_AXIS: Axis = Axis(2);

impl ImageArrayResponse {
    pub fn rank(&self) -> ImageArrayResponseRank {
        match self.data.len_of(COLOUR_AXIS) {
            1 => ImageArrayResponseRank::Rank2,
            _ => ImageArrayResponseRank::Rank3,
        }
    }

    fn value_as_serialize(&self) -> impl '_ + Serialize {
        #[derive(Serialize)]
        #[serde(untagged)]
        enum Value<'img> {
            Rank2(#[serde(with = "serde_ndim")] ndarray::ArrayView2<'img, i32>),
            Rank3(#[serde(with = "serde_ndim")] ndarray::ArrayView3<'img, i32>),
        }

        let data = self.data.view();

        match self.rank() {
            ImageArrayResponseRank::Rank2 => Value::Rank2(data.remove_axis(COLOUR_AXIS)),
            ImageArrayResponseRank::Rank3 => Value::Rank3(data),
        }
    }
}

#[cfg(not(target_endian = "little"))]
compile_error!(
"Image handling is currently only supported on little-endian platforms for simplicity & performance.
If you have a real-world use case for big-endian support, please open an issue on GitHub."
);

#[cfg(any(feature = "client", feature = "server"))]
#[repr(C)]
#[derive(Clone, Copy, Zeroable, Pod)]
struct ImageBytesMetadata {
    metadata_version: i32,
    error_number: i32,
    client_transaction_id: Option<NonZeroU32>,
    server_transaction_id: Option<NonZeroU32>,
    data_start: i32,
    image_element_type: i32,
    transmission_element_type: i32,
    rank: i32,
    dimension_1: i32,
    dimension_2: i32,
    dimension_3: i32,
}

const IMAGE_BYTES_TYPE: &str = "application/imagebytes";

#[cfg(feature = "server")]
impl ImageArrayResponse {
    pub(crate) fn is_accepted(headers: &axum::headers::HeaderMap) -> bool {
        use mediatype::{MediaType, MediaTypeList};

        const MEDIA_TYPE: MediaType<'static> = MediaType::new(
            mediatype::names::APPLICATION,
            mediatype::Name::new_unchecked("imagebytes"),
        );

        headers
            .get_all(axum::http::header::ACCEPT)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .flat_map(MediaTypeList::new)
            .filter_map(std::result::Result::ok)
            .any(|media_type| media_type.essence() == MEDIA_TYPE)
    }
}

#[cfg(feature = "server")]
pub(crate) struct ImageBytesResponse(pub(crate) ImageArrayResponse);

#[cfg(feature = "server")]
impl crate::server::Response for ASCOMResult<ImageBytesResponse> {
    fn into_axum(
        self,
        transaction: crate::server::ResponseTransaction,
    ) -> axum::response::Response {
        use axum::response::IntoResponse;

        let mut metadata = ImageBytesMetadata {
            metadata_version: 1,
            data_start: i32::try_from(std::mem::size_of::<ImageBytesMetadata>())
                .expect("internal error: metadata size is too large"),
            client_transaction_id: transaction.client_transaction_id,
            server_transaction_id: Some(transaction.server_transaction_id),
            ..Zeroable::zeroed()
        };
        let data = match &self {
            Ok(ImageBytesResponse(ImageArrayResponse { data })) => {
                metadata.image_element_type = ImageArrayResponseType::Integer as i32;
                metadata.transmission_element_type = ImageArrayResponseType::Integer as i32;
                let dims = {
                    let (dim0, dim1, dim2) = data.dim();
                    [dim0, dim1, dim2]
                        .map(|dim| i32::try_from(dim).expect("dimension is too large"))
                };
                metadata.dimension_1 = dims[0];
                metadata.dimension_2 = dims[1];
                metadata.rank = match dims[2] {
                    1_i32 => 2_i32,
                    n => {
                        metadata.dimension_3 = n;
                        3_i32
                    }
                };
                bytemuck::cast_slice(
                    data.as_slice()
                        .expect("internal arrays should always be in standard layout"),
                )
            }
            Err(err) => {
                metadata.error_number = err.code.raw().into();
                err.message.as_bytes()
            }
        };
        let mut bytes = Vec::with_capacity(std::mem::size_of::<ImageBytesMetadata>() + data.len());
        bytes.extend_from_slice(bytemuck::bytes_of(&metadata));
        bytes.extend_from_slice(data);
        (
            [(axum::http::header::CONTENT_TYPE, IMAGE_BYTES_TYPE)],
            bytes,
        )
            .into_response()
    }
}

#[cfg(feature = "client")]
const _: () = {
    use crate::client::{Response, ResponseTransaction, ResponseWithTransaction};
    use crate::{ASCOMError, ASCOMErrorCode};
    use bytes::Bytes;
    use mime::Mime;

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
};

impl Serialize for ImageArrayResponse {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        #[serde(rename_all = "PascalCase")]
        struct Repr<Value> {
            #[serde(rename = "Type")]
            type_: ImageArrayResponseType,
            rank: ImageArrayResponseRank,
            value: Value,
        }

        Repr {
            type_: ImageArrayResponseType::Integer,
            rank: self.rank(),
            value: self.value_as_serialize(),
        }
        .serialize(serializer)
    }
}

struct ResponseVisitor;

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
