use serde::de::{DeserializeOwned, DeserializeSeed, Visitor};
use std::marker::PhantomData;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum ImageArrayResponseRank {
    Mono = 2,
    Rgb = 3,
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
    pub rank: ImageArrayResponseRank,
    pub height: u32,
    pub flat_data: Vec<T>,
}

impl<T: ImageArrayResponseNumber> ImageArrayTypedResponse<T> {
    #[auto_enums::auto_enum(serde::Serialize)]
    fn value_as_serialize(&self) -> impl '_ + Serialize {
        #[derive(Serialize)]
        struct IterSerialize<I: Iterator + Clone>(#[serde(with = "serde_iter::seq")] I)
        where
            I::Item: Serialize;

        let rows_iter = self.flat_data.chunks_exact(self.height as usize);
        match self.rank {
            ImageArrayResponseRank::Mono => IterSerialize(rows_iter),
            ImageArrayResponseRank::Rgb => IterSerialize(rows_iter.map(|row| {
                IterSerialize(
                    // TODO: use array_chunks when stabilized
                    row.chunks_exact(3).map(|rgb| {
                        #[allow(clippy::unwrap_used)]
                        <&[T; 3]>::try_from(rgb).unwrap()
                    }),
                )
            })),
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
            rank: self.rank,
            value: self.value_as_serialize(),
        }
        .serialize(serializer)
    }
}

struct ValueVisitor<'data, T> {
    dest: &'data mut Vec<T>,
    rank: u8,
}

impl<'de, T: ImageArrayResponseNumber> Visitor<'de> for ValueVisitor<'_, T> {
    type Value = u32;

    fn expecting(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(fmt, "a {}-dimensional array", self.rank)
    }

    fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        if self.rank > 1 {
            let mut count = 0;
            while seq
                .next_element_seed(ValueVisitor {
                    dest: self.dest,
                    rank: self.rank - 1,
                })?
                .is_some()
            {
                count += 1;
            }
            Ok(count)
        } else {
            // TODO: add size hints based on the first row for more optimal allocation.
            let len_before = self.dest.len();
            while let Some(value) = seq.next_element::<T>()? {
                self.dest.push(value);
            }
            Ok((self.dest.len() - len_before) as u32)
        }
    }
}

impl<'de, T: ImageArrayResponseNumber> DeserializeSeed<'de> for ValueVisitor<'_, T> {
    type Value = u32;

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
        let mut flat_data = Vec::new();
        let height = map.next_value_seed(ValueVisitor {
            dest: &mut flat_data,
            rank: rank as u8,
        })?;

        Ok(ImageArrayTypedResponse {
            rank,
            height,
            flat_data,
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
