use serde::de::{DeserializeSeed, Error, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};

struct DeviceStateDeserializer<D>(D);

impl<'de, V: Visitor<'de>> Visitor<'de> for DeviceStateDeserializer<V> {
    type Value = V::Value;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a sequence")
    }

    fn visit_seq<S: SeqAccess<'de>>(self, seq: S) -> Result<Self::Value, S::Error> {
        self.0.visit_map(DeviceStateDeserializer(seq))
    }
}

impl<'de, A: SeqAccess<'de>> MapAccess<'de> for DeviceStateDeserializer<A> {
    type Error = A::Error;

    fn next_key_seed<K: DeserializeSeed<'de>>(
        &mut self,
        _seed: K,
    ) -> Result<Option<K::Value>, Self::Error> {
        unimplemented!("currently we only support whole entries for simplicity")
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(
        &mut self,
        _seed: V,
    ) -> Result<V::Value, Self::Error> {
        unimplemented!("currently we only support whole entries for simplicity")
    }

    fn next_entry_seed<K: DeserializeSeed<'de>, V: DeserializeSeed<'de>>(
        &mut self,
        name: K,
        value: V,
    ) -> Result<Option<(K::Value, V::Value)>, Self::Error> {
        struct Entry<K, V> {
            name: K,
            value: V,
        }

        impl<'de, K: DeserializeSeed<'de>, V: DeserializeSeed<'de>> DeserializeSeed<'de> for Entry<K, V> {
            type Value = (K::Value, V::Value);

            fn deserialize<D: Deserializer<'de>>(
                self,
                deserializer: D,
            ) -> Result<Self::Value, D::Error> {
                deserializer.deserialize_map(self)
            }
        }

        impl<'de, K: DeserializeSeed<'de>, V: DeserializeSeed<'de>> Visitor<'de> for Entry<K, V> {
            type Value = (K::Value, V::Value);

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a {Name, Value} object")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                #[derive(Deserialize)]
                #[serde(field_identifier)]
                enum Field {
                    Name,
                    Value,
                }

                struct FieldState<'de, S: DeserializeSeed<'de>> {
                    name: &'static str,
                    seed: Option<S>,
                    value: Option<S::Value>,
                }

                impl<'de, S: DeserializeSeed<'de>> DeserializeSeed<'de> for &'_ mut FieldState<'de, S> {
                    type Value = ();

                    fn deserialize<D: Deserializer<'de>>(
                        self,
                        deserializer: D,
                    ) -> Result<Self::Value, D::Error> {
                        let seed = self
                            .seed
                            .take()
                            .ok_or_else(|| Error::duplicate_field(self.name))?;
                        self.value = Some(seed.deserialize(deserializer)?);
                        Ok(())
                    }
                }

                impl<'de, S: DeserializeSeed<'de>> FieldState<'de, S> {
                    const fn new(name: &'static str, seed: S) -> Self {
                        Self {
                            name,
                            seed: Some(seed),
                            value: None,
                        }
                    }

                    fn finish<E: Error>(self) -> Result<S::Value, E> {
                        self.value.ok_or_else(|| Error::missing_field(self.name))
                    }
                }

                let mut name = FieldState::new("Name", self.name);
                let mut value = FieldState::new("Value", self.value);

                while let Some(field) = map.next_key::<Field>()? {
                    match field {
                        Field::Name => map.next_value_seed(&mut name),
                        Field::Value => map.next_value_seed(&mut value),
                    }?;
                }

                Ok((name.finish()?, value.finish()?))
            }
        }

        self.0.next_element_seed(Entry { name, value })
    }
}

impl<'de, D: Deserializer<'de>> Deserializer<'de> for DeviceStateDeserializer<D> {
    type Error = D::Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.0.deserialize_seq(DeviceStateDeserializer(visitor))
    }

    serde::forward_to_deserialize_any![
        char
        bool
        i8
        i16
        i32
        i64
        u8
        u16
        u32
        u64
        f32
        f64
        str
        string
        bytes
        byte_buf
        option
        unit
        unit_struct
        newtype_struct
        seq
        tuple
        tuple_struct
        map
        struct
        enum
        identifier
        ignored_any
    ];
}

pub(crate) struct TimestampedDeviceStateRepr<T>(T);

impl<'de, T: Deserialize<'de>> Deserialize<'de> for TimestampedDeviceStateRepr<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        T::deserialize(DeviceStateDeserializer(deserializer)).map(Self)
    }
}

impl<T> TimestampedDeviceStateRepr<T> {
    pub(crate) fn into_inner(self) -> T {
        self.0
    }
}
