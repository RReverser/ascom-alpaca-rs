//! This is an alternative to the problematic `serde(flatten)` attribute
//! that efficiently deserializes large structs by caching only the small flattened
//! part instead of the whole JSON object.
//!
//! This makes a particularly big difference for the image array JSON representation.

use serde::de::value::MapDeserializer;
use serde::de::{
    Deserialize, DeserializeOwned, DeserializeSeed, Deserializer, IntoDeserializer, MapAccess,
    Visitor,
};
use serde::forward_to_deserialize_any;
use std::borrow::Cow;
use std::marker::PhantomData;
use thiserror::Error;

struct LearnNamesOfSmallType;

#[derive(Error, Debug)]
#[error("{self:?}")]
enum Status {
    Success(&'static [&'static str]),
    WrongType,
}

impl serde::de::Error for Status {
    fn custom<T: std::fmt::Display>(_msg: T) -> Self {
        Self::WrongType
    }
}

impl<'de> Deserializer<'de> for LearnNamesOfSmallType {
    type Error = Status;

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        fields: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Self::Error> {
        // Representing as error to force Serde to bail out early
        // instead of attempting to actually parse the struct contents.
        Err(Status::Success(fields))
    }

    fn deserialize_any<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Self::Error> {
        Err(Status::WrongType)
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes byte_buf
        option unit unit_struct newtype_struct seq tuple tuple_struct map enum identifier ignored_any
    }
}

struct MapAccessForLargeType<'de, 'buf, M> {
    inner_map_access: M,
    small_type_fields: &'static [&'static str],
    small_type_values: &'buf mut [Option<&'de serde_json::value::RawValue>],
}

impl<'de, M: MapAccess<'de>> MapAccess<'de> for MapAccessForLargeType<'de, '_, M> {
    type Error = M::Error;

    fn next_key_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Self::Error> {
        while let Some(key) = self.inner_map_access.next_key::<Cow<'de, str>>()? {
            if let Some(index) = self
                .small_type_fields
                .iter()
                .position(|&field| field == key)
            {
                let value = self.inner_map_access.next_value()?;
                let prev = self.small_type_values[index].replace(value);
                debug_assert!(prev.is_none(), "duplicate key");
            } else {
                return seed.deserialize(key.into_deserializer()).map(Some);
            }
        }
        Ok(None)
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(
        &mut self,
        seed: V,
    ) -> Result<V::Value, Self::Error> {
        self.inner_map_access.next_value_seed(seed)
    }
}

impl<'de, M: MapAccess<'de>> Deserializer<'de> for MapAccessForLargeType<'de, '_, M> {
    type Error = M::Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_map(self)
    }

    // This is one reason we can't use `MapAccessDeserializer`: flattening should support `()` as a target.
    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let _ = self.deserialize_any(serde::de::IgnoredAny)?;
        visitor.visit_unit()
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes byte_buf
        option unit_struct newtype_struct seq tuple tuple_struct map enum identifier ignored_any
        struct
    }
}

struct FlattenedVisitor<S, L> {
    small_type_fields: &'static [&'static str],
    _phantom: PhantomData<fn() -> Flattened<S, L>>,
}

impl<'de, S: DeserializeOwned, L: Deserialize<'de>> Visitor<'de> for FlattenedVisitor<S, L> {
    type Value = Flattened<S, L>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a flattened struct")
    }

    fn visit_map<M: MapAccess<'de>>(self, map: M) -> Result<Self::Value, M::Error> {
        let mut small_type_values = vec![None; self.small_type_fields.len()];

        let large_type = L::deserialize(MapAccessForLargeType {
            inner_map_access: map,
            small_type_fields: self.small_type_fields,
            small_type_values: &mut small_type_values,
        })?;

        let small_type = S::deserialize(MapDeserializer::new(
            self.small_type_fields
                .iter()
                .zip(small_type_values)
                .filter_map(|(&key, value)| Some((key, value?))),
        ))
        .map_err(serde::de::Error::custom)?;

        Ok(Flattened(small_type, large_type))
    }
}

#[derive(Debug)]
pub(crate) struct Flattened<S, L>(pub(crate) S, pub(crate) L);

impl<'de, S: DeserializeOwned, L: Deserialize<'de>> Deserialize<'de> for Flattened<S, L> {
    #[allow(clippy::panic_in_result_fn)]
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let small_type_fields = match S::deserialize(LearnNamesOfSmallType) {
            Ok(_) => unreachable!(),
            Err(Status::Success(small_type_fields)) => small_type_fields,
            Err(Status::WrongType) => {
                return Err(serde::de::Error::custom("First type must be a struct"));
            }
        };

        deserializer.deserialize_map(FlattenedVisitor {
            small_type_fields,
            _phantom: PhantomData,
        })
    }
}
