use impl_serialize::impl_serialize;
use serde::ser::{self, Serialize, SerializeSeq, Serializer};

struct DeviceStateSerializer<S>(S);

impl<S: Serializer> Serializer for DeviceStateSerializer<S> {
    type Ok = S::Ok;
    type Error = S::Error;

    type SerializeMap = DeviceStateSerializer<S::SerializeSeq>;
    type SerializeSeq = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeStruct = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeStructVariant = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeTuple = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeTupleStruct = ser::Impossible<Self::Ok, Self::Error>;
    type SerializeTupleVariant = ser::Impossible<Self::Ok, Self::Error>;

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        self.0.serialize_seq(len).map(DeviceStateSerializer)
    }

    impl_serialize!(
        Err(ser::Error::custom("device state must be a struct")),
        [
            bool,
            char,
            bytes,
            i8,
            i16,
            i32,
            i64,
            u8,
            u16,
            u32,
            u64,
            f32,
            f64,
            str,
            none,
            some,
            unit,
            unit_struct,
            unit_variant,
            newtype_struct,
            newtype_variant,
            seq,
            struct,
            tuple,
            tuple_struct,
            tuple_variant,
            struct_variant
        ]
    );
}

impl<S: SerializeSeq> ser::SerializeMap for DeviceStateSerializer<S> {
    type Ok = S::Ok;
    type Error = S::Error;

    fn serialize_key<T>(&mut self, _key: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        unimplemented!("currently we only support whole entries for simplicity")
    }

    fn serialize_value<T>(&mut self, _value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + Serialize,
    {
        unimplemented!("currently we only support whole entries for simplicity")
    }

    fn serialize_entry<K, V>(&mut self, key: &K, value: &V) -> Result<(), Self::Error>
    where
        K: ?Sized + Serialize,
        V: ?Sized + Serialize,
    {
        #[derive(serde::Serialize)]
        #[serde(rename_all = "PascalCase")]
        struct Entry<K, V> {
            name: K,
            value: V,
        }

        self.0.serialize_element(&Entry { name: key, value })
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        self.0.end()
    }
}

pub(crate) fn serialize<S: Serializer>(
    state: &impl Serialize,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    state.serialize(DeviceStateSerializer(serializer))
}
