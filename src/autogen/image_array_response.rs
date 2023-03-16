use super::ImageArrayResponseType;
use serde::de::{DeserializeSeed, Visitor};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum ImageArrayResponseRank {
    Rank2 = 2,
    Rank3 = 3,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ImageArrayResponse {
    pub data: ndarray::Array3<i32>,
}

impl ImageArrayResponse {
    pub fn rank(&self) -> ImageArrayResponseRank {
        match self.data.len_of(ndarray::Axis(2)) {
            1 => ImageArrayResponseRank::Rank2,
            _ => ImageArrayResponseRank::Rank3,
        }
    }

    #[auto_enums::auto_enum(serde::Serialize)]
    fn value_as_serialize(&self) -> impl '_ + Serialize {
        #[derive(Serialize)]
        struct IterSerialize<I: Iterator + Clone>(#[serde(with = "serde_iter::seq")] I)
        where
            I::Item: Serialize;

        match self.rank() {
            ImageArrayResponseRank::Rank2 => IterSerialize(self.data.outer_iter().map(|column| {
                column
                    .to_slice()
                    .expect("internal arrays should always be in standard layout")
            })),
            ImageArrayResponseRank::Rank3 => IterSerialize(
                // Slicing is used as a workaround for https://github.com/rust-ndarray/ndarray/issues/1232.
                (0..self.data.len_of(ndarray::Axis(0))).map(move |column_i| {
                    IterSerialize((0..self.data.len_of(ndarray::Axis(1))).map(move |row_i| {
                        self.data
                            .slice(ndarray::s![column_i, row_i, ..])
                            .to_slice()
                            .expect("internal arrays should always be in standard layout")
                    }))
                }),
            ),
        }
    }
}

// For now only implementing where endianness matches the expected little one
// to avoid dealing with order conversions.
#[cfg(target_endian = "little")]
mod image_bytes {
    use super::ImageArrayResponse;
    use crate::api::ImageArrayResponseType;
    use crate::response::OpaqueResponse;
    use crate::transaction::{ClientResponseTransaction, Response, ServerResponseTransaction};
    use crate::{ASCOMError, ASCOMErrorCode, ASCOMResult};
    use axum::response::IntoResponse;
    use bytemuck::{Pod, Zeroable};
    use bytes::Bytes;
    use mime::Mime;

    #[repr(C)]
    #[derive(Clone, Copy, Zeroable, Pod)]
    struct ImageBytesMetadata {
        metadata_version: i32,
        error_number: i32,
        client_transaction_id: u32,
        server_transaction_id: u32,
        data_start: i32,
        image_element_type: i32,
        transmission_element_type: i32,
        rank: i32,
        dimension_1: i32,
        dimension_2: i32,
        dimension_3: i32,
    }

    impl Response for ASCOMResult<ImageArrayResponse> {
        fn into_axum(self, transaction: ServerResponseTransaction) -> axum::response::Response {
            let mut metadata = ImageBytesMetadata {
                metadata_version: 1,
                #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
                data_start: std::mem::size_of::<ImageBytesMetadata>() as i32,
                client_transaction_id: transaction.client_transaction_id.unwrap_or(0),
                server_transaction_id: transaction.server_transaction_id,
                ..Zeroable::zeroed()
            };
            let data = match &self {
                Ok(this) => {
                    metadata.image_element_type = ImageArrayResponseType::Integer as i32;
                    metadata.transmission_element_type = ImageArrayResponseType::Integer as i32;
                    let dims = <[usize; 3]>::try_from(this.data.shape())
                        .expect("dimension count mismatch")
                        .map(|dim| i32::try_from(dim).expect("dimension is too large"));
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
                        this.data
                            .as_slice()
                            .expect("internal arrays should always be in standard layout"),
                    )
                }
                Err(err) => {
                    metadata.error_number = err.code.0.into();
                    err.message.as_bytes()
                }
            };
            let mut bytes =
                Vec::with_capacity(std::mem::size_of::<ImageBytesMetadata>() + data.len());
            bytes.extend_from_slice(bytemuck::bytes_of(&metadata));
            bytes.extend_from_slice(data);
            (
                [(axum::http::header::CONTENT_TYPE, "application/imagebytes")],
                bytes,
            )
                .into_response()
        }

        fn prepare_reqwest(request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
            request.header("Accept", "application/imagebytes")
        }

        fn from_reqwest(
            mime_type: Mime,
            bytes: Bytes,
        ) -> anyhow::Result<(ClientResponseTransaction, Self)> {
            if mime_type.essence_str() != "application/imagebytes" {
                let (transaction, opaque_response_res) =
                    <ASCOMResult<OpaqueResponse>>::from_reqwest(mime_type, bytes)?;

                let response = opaque_response_res.and_then(|opaque_response| {
                    opaque_response.try_as().map_err(|err| {
                        ASCOMError::new(ASCOMErrorCode::UNSPECIFIED, format!("{err:#}"))
                    })
                });

                return Ok((transaction, response));
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
            let transaction = ClientResponseTransaction {
                client_transaction_id: if metadata.client_transaction_id == 0 {
                    None
                } else {
                    Some(metadata.client_transaction_id)
                },
                server_transaction_id: if metadata.server_transaction_id == 0 {
                    None
                } else {
                    Some(metadata.server_transaction_id)
                },
            };
            let ascom_result = if metadata.error_number == 0_i32 {
                anyhow::ensure!(
                    metadata.image_element_type == ImageArrayResponseType::Integer as i32,
                    "only Integer image element type is supported, got {}",
                    metadata.image_element_type
                );
                anyhow::ensure!(
                    metadata.transmission_element_type == ImageArrayResponseType::Integer as i32,
                    "only Integer transmission element type is supported for now, got {}",
                    metadata.transmission_element_type
                );
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
                    data: ndarray::Array::from_shape_vec(
                        shape,
                        bytemuck::cast_vec(data.to_owned()),
                    )
                    .expect("couldn't match the parsed shape to the data"),
                })
            } else {
                Err(ASCOMError::new(
                    ASCOMErrorCode(u16::try_from(metadata.error_number)?),
                    std::str::from_utf8(data)?.to_owned(),
                ))
            };
            Ok((transaction, ascom_result))
        }
    }
}

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

struct ValueVisitorCtx {
    dest: Vec<i32>,
    first_pass: bool,
}

impl ValueVisitorCtx {
    const fn new() -> Self {
        Self {
            dest: Vec::new(),
            first_pass: true,
        }
    }
}

struct ValueVisitor<'data> {
    ctx: &'data mut ValueVisitorCtx,
    shape: &'data mut [usize],
}

impl<'de> Visitor<'de> for ValueVisitor<'_> {
    type Value = ();

    fn expecting(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(fmt, "a {}-dimensional array", self.shape.len())
    }

    fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let (current_shape_len, rest_shape) = self
            .shape
            .split_first_mut()
            .expect("rank should never reach zero");
        // Store the value of the first pass as the recursion will overwrite it.
        let first_pass = self.ctx.first_pass;
        let actual_len = if rest_shape.is_empty() {
            // Reached the innermost dimension, unmark the first pass.
            self.ctx.first_pass = false;
            let len_before = self.ctx.dest.len();
            while let Some(value) = seq.next_element()? {
                self.ctx.dest.push(value);
            }
            self.ctx.dest.len() - len_before
        } else {
            let mut actual_len = 0;
            while seq
                .next_element_seed(ValueVisitor {
                    ctx: self.ctx,
                    shape: rest_shape,
                })?
                .is_some()
            {
                actual_len += 1;
            }
            actual_len
        };
        if first_pass {
            // If this is the first pass, what we've seen determines the shape for all other passes.
            *current_shape_len = actual_len;
        } else if *current_shape_len != actual_len {
            // Otherwise, we check that the shape is consistent with the first one.
            return Err(serde::de::Error::invalid_length(
                actual_len,
                &current_shape_len.to_string().as_str(),
            ));
        }
        Ok(())
    }
}

impl<'de> DeserializeSeed<'de> for ValueVisitor<'_> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(self)
    }
}

struct ResponseVisitor;

fn expect_key<'de, A: serde::de::MapAccess<'de>>(
    map: &mut A,
    expected_key: &'static str,
) -> Result<(), A::Error> {
    match map.next_key::<&str>()? {
        Some(key) if key == expected_key => Ok(()),
        Some(key) => Err(serde::de::Error::custom(format!(
            "expected field {expected_key}, got {key}"
        ))),
        None => Err(serde::de::Error::missing_field(expected_key)),
    }
}

impl<'de> Visitor<'de> for ResponseVisitor {
    type Value = ImageArrayResponse;

    fn expecting(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.write_str("a map")
    }

    fn visit_map<A: serde::de::MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        expect_key(&mut map, "Type")?;
        let type_ = map.next_value::<ImageArrayResponseType>()?;
        if type_ != ImageArrayResponseType::Integer {
            return Err(serde::de::Error::custom(format!(
                r"expected Type == Integer, got {type_:?}",
            )));
        }

        expect_key(&mut map, "Rank")?;
        let rank = map.next_value::<ImageArrayResponseRank>()?;

        expect_key(&mut map, "Value")?;
        let mut shape = [0, 0, 1];
        let mut ctx = ValueVisitorCtx::new();
        map.next_value_seed(ValueVisitor {
            ctx: &mut ctx,
            shape: &mut shape[..rank as usize],
        })?;

        Ok(ImageArrayResponse {
            data: ndarray::Array::from_shape_vec(
                ndarray::Ix3(shape[0], shape[1], shape[2]),
                ctx.dest,
            )
            .expect("couldn't match the parsed shape to the data"),
        })
    }
}

impl<'de> Deserialize<'de> for ImageArrayResponse {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_map(ResponseVisitor)
    }
}
