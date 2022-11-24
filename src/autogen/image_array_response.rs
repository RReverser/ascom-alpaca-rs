use super::ImageArrayResponseType;
use serde::de::{DeserializeOwned, DeserializeSeed, Visitor};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::marker::PhantomData;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum ImageArrayResponseRank {
    Rank2 = 2,
    Rank3 = 3,
}

pub trait ImageArrayResponseNumber: Serialize + DeserializeOwned {
    const TYPE: ImageArrayResponseType;
}

impl ImageArrayResponseNumber for i16 {
    const TYPE: ImageArrayResponseType = ImageArrayResponseType::Short;
}

impl ImageArrayResponseNumber for i32 {
    const TYPE: ImageArrayResponseType = ImageArrayResponseType::Integer;
}

impl ImageArrayResponseNumber for f64 {
    const TYPE: ImageArrayResponseType = ImageArrayResponseType::Double;
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ImageArrayTypedResponse<T: ImageArrayResponseNumber> {
    pub data: ndarray::Array3<T>,
}

impl<T: ImageArrayResponseNumber> ImageArrayTypedResponse<T> {
    fn rank(&self) -> ImageArrayResponseRank {
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

impl<T: ImageArrayResponseNumber> Serialize for ImageArrayTypedResponse<T> {
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
            type_: T::TYPE,
            rank: self.rank(),
            value: self.value_as_serialize(),
        }
        .serialize(serializer)
    }
}

struct ValueVisitorCtx<T> {
    dest: Vec<T>,
    first_pass: bool,
}

impl<T> ValueVisitorCtx<T> {
    const fn new() -> Self {
        Self {
            dest: Vec::new(),
            first_pass: true,
        }
    }
}

struct ValueVisitor<'data, T> {
    ctx: &'data mut ValueVisitorCtx<T>,
    shape: &'data mut [usize],
}

impl<'de, T: ImageArrayResponseNumber> Visitor<'de> for ValueVisitor<'_, T> {
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
            while let Some(value) = seq.next_element::<T>()? {
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

impl<'de, T: ImageArrayResponseNumber> DeserializeSeed<'de> for ValueVisitor<'_, T> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(self)
    }
}

struct ResponseVisitor<T> {
    expect_type: bool,
    _phantom: PhantomData<T>,
}

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

impl<'de, T: ImageArrayResponseNumber> Visitor<'de> for ResponseVisitor<T> {
    type Value = ImageArrayTypedResponse<T>;

    fn expecting(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.write_str("a map")
    }

    fn visit_map<A: serde::de::MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        // If `expect_type` is `false`, it means the `Type` field was already parsed by `ImageArrayVariantResponse`.
        // If there is one, check that it matches the expectation.
        if self.expect_type {
            expect_key(&mut map, "Type")?;
            let type_ = map.next_value::<ImageArrayResponseType>()?;
            if type_ != T::TYPE {
                return Err(serde::de::Error::custom(format!(
                    "expected type {:?}, got {:?}",
                    T::TYPE,
                    type_
                )));
            }
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

        Ok(ImageArrayTypedResponse {
            data: ndarray::Array::from_shape_vec(
                ndarray::Ix3(shape[0], shape[1], shape[2]),
                ctx.dest,
            )
            .expect("internal error: couldn't match the parsed shape to the data"),
        })
    }
}

impl<'de, T: ImageArrayResponseNumber> Deserialize<'de> for ImageArrayTypedResponse<T> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_map(ResponseVisitor {
            expect_type: true,
            _phantom: PhantomData,
        })
    }
}

pub type ImageArrayResponse = ImageArrayTypedResponse<i32>;

#[derive(Debug, PartialEq, Eq, Clone, Serialize)]
#[serde(untagged)]
pub enum ImageArrayVariantResponse {
    Short(ImageArrayTypedResponse<i16>),
    Integer(ImageArrayTypedResponse<i32>),
    Double(ImageArrayTypedResponse<f64>),
}

struct VariantResponseVisitor;

fn visit_image_variant<'de, T: ImageArrayResponseNumber, A: serde::de::MapAccess<'de>>(
    map: A,
    to_variant: fn(ImageArrayTypedResponse<T>) -> ImageArrayVariantResponse,
) -> Result<ImageArrayVariantResponse, A::Error> {
    ResponseVisitor {
        expect_type: false,
        _phantom: PhantomData,
    }
    .visit_map(map)
    .map(to_variant)
}

impl<'de> Visitor<'de> for VariantResponseVisitor {
    type Value = ImageArrayVariantResponse;

    fn expecting(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.write_str("a map")
    }

    fn visit_map<A: serde::de::MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        expect_key(&mut map, "Type")?;
        let type_ = map.next_value::<ImageArrayResponseType>()?;

        match type_ {
            ImageArrayResponseType::Unknown => {
                Err(serde::de::Error::custom("received unknown image data type"))
            }
            ImageArrayResponseType::Short => {
                visit_image_variant(map, ImageArrayVariantResponse::Short)
            }
            ImageArrayResponseType::Integer => {
                visit_image_variant(map, ImageArrayVariantResponse::Integer)
            }
            ImageArrayResponseType::Double => {
                visit_image_variant(map, ImageArrayVariantResponse::Double)
            }
        }
    }
}

impl<'de> Deserialize<'de> for ImageArrayVariantResponse {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_map(VariantResponseVisitor)
    }
}
