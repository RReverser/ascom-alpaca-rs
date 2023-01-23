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
    use crate::{ASCOMError, ASCOMErrorCode, ASCOMResult};
    use bytemuck::{Pod, Zeroable};

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

    impl ImageArrayResponse {
        pub fn from_image_bytes(bytes: &[u8]) -> anyhow::Result<ASCOMResult<Self>> {
            let metadata = bytes
                .get(..std::mem::size_of::<ImageBytesMetadata>())
                .ok_or_else(|| anyhow::anyhow!("not enough bytes to read image metadata"))?;
            let metadata = bytemuck::pod_read_unaligned::<ImageBytesMetadata>(metadata);
            anyhow::ensure!(
                metadata.metadata_version == 1,
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
            if metadata.error_number != 0 {
                return Ok(Err(ASCOMError::new(
                    ASCOMErrorCode(u16::try_from(metadata.error_number)?),
                    std::str::from_utf8(data)?.to_owned(),
                )));
            }
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
                            metadata.dimension_3 == 0,
                            "dimension 3 must be 0 for rank 2, got {}",
                            metadata.dimension_3
                        );
                        1
                    }
                    3 => usize::try_from(metadata.dimension_3)?,
                    rank => anyhow::bail!("unsupported rank {}, expected 2 or 3", rank),
                },
            );
            Ok(Ok(ImageArrayResponse {
                data: ndarray::Array::from_shape_vec(shape, bytemuck::cast_vec(data.to_owned()))
                    .expect("internal error: couldn't match the parsed shape to the data"),
            }))
        }

        pub fn to_image_bytes(this: ASCOMResult<&Self>) -> Vec<u8> {
            let mut metadata = ImageBytesMetadata {
                metadata_version: 1,
                data_start: std::mem::size_of::<ImageBytesMetadata>() as i32,
                ..Zeroable::zeroed()
            };
            let data = match this {
                Ok(this) => {
                    metadata.image_element_type = ImageArrayResponseType::Integer as i32;
                    metadata.transmission_element_type = ImageArrayResponseType::Integer as i32;
                    let dims = <[usize; 3]>::try_from(this.data.shape())
                        .expect("dimension count mismatch")
                        .map(|dim| i32::try_from(dim).expect("dimension is too large"));
                    metadata.dimension_1 = dims[0];
                    metadata.dimension_2 = dims[1];
                    metadata.rank = match dims[2] {
                        1 => 2,
                        n => {
                            metadata.dimension_3 = n;
                            3
                        }
                    };
                    bytemuck::cast_slice(
                        this.data
                            .as_slice()
                            .expect("internal arrays should always be in standard layout"),
                    )
                }
                Err(ref err) => {
                    metadata.error_number = err.code.0.into();
                    err.message.as_bytes()
                }
            };
            let mut bytes =
                Vec::with_capacity(std::mem::size_of::<ImageBytesMetadata>() + data.len());
            bytes.extend_from_slice(bytemuck::bytes_of(&metadata));
            bytes.extend_from_slice(data);
            bytes
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
                actual_len as usize,
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
            "expected field {}, got {}",
            expected_key, key
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
            .expect("internal error: couldn't match the parsed shape to the data"),
        })
    }
}

impl<'de> Deserialize<'de> for ImageArrayResponse {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_map(ResponseVisitor)
    }
}
