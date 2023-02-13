use super::rpc::OpaqueResponse;
use crate::ASCOMResult;
use axum::response::{IntoResponse, Response};
use axum::Json;
use indexmap::IndexMap;
use serde::de::value::{BorrowedStrDeserializer, UnitDeserializer};
use serde::de::DeserializeOwned;
use serde::{forward_to_deserialize_any, Deserialize, Deserializer, Serialize};
use std::fmt::Debug;
use std::sync::atomic::AtomicU32;

#[derive(Serialize, Deserialize)]
pub(crate) struct TransactionIds {
    #[serde(rename = "ClientID")]
    #[serde(skip_serializing)]
    #[serde(default)]
    #[allow(dead_code)]
    pub(crate) client_id: Option<u32>,
    #[serde(rename = "ClientTransactionID")]
    #[serde(default)]
    pub(crate) client_transaction_id: Option<u32>,
    #[serde(rename = "ServerTransactionID")]
    #[serde(skip_deserializing)]
    #[serde(default = "generate_server_transaction_id")]
    pub(crate) server_transaction_id: u32,
}

impl TransactionIds {
    pub(crate) fn span(&self) -> tracing::Span {
        tracing::debug_span!(
            "alpaca_transaction",
            client_id = self.client_id,
            client_transaction_id = self.client_transaction_id,
            server_transaction_id = self.server_transaction_id,
        )
    }

    pub(crate) const fn make_response(self, result: ASCOMResult<OpaqueResponse>) -> ASCOMResponse {
        ASCOMResponse {
            transaction: self,
            result,
        }
    }
}

fn generate_server_transaction_id() -> u32 {
    static SERVER_TRANSACTION_ID: AtomicU32 = AtomicU32::new(0);
    SERVER_TRANSACTION_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

#[derive(Serialize)]
#[repr(transparent)]
struct CaseInsensitiveStr(str);

impl AsRef<CaseInsensitiveStr> for str {
    fn as_ref(&self) -> &CaseInsensitiveStr {
        #[allow(clippy::as_conversions)]
        unsafe {
            &*(self as *const _ as *const _)
        }
    }
}

impl From<Box<str>> for Box<CaseInsensitiveStr> {
    fn from(str: Box<str>) -> Self {
        let as_ptr = Box::into_raw(str);
        #[allow(clippy::as_conversions)]
        unsafe {
            Self::from_raw(as_ptr as *mut _)
        }
    }
}

impl<'de> Deserialize<'de> for Box<CaseInsensitiveStr> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        <Box<str>>::deserialize(deserializer).map(Into::into)
    }
}

impl Debug for CaseInsensitiveStr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl PartialEq for CaseInsensitiveStr {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq_ignore_ascii_case(&other.0)
    }
}

impl Eq for CaseInsensitiveStr {}

impl std::hash::Hash for CaseInsensitiveStr {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        for b in self.0.as_bytes() {
            state.write_u8(b.to_ascii_lowercase());
        }
        state.write_u8(0xff);
    }
}

#[derive(Debug, Deserialize)]
#[serde(transparent)]
pub struct ASCOMParams(IndexMap<Box<CaseInsensitiveStr>, String>);

struct ParamDeserializer<E> {
    str: String,
    phantom: std::marker::PhantomData<fn() -> E>,
}

impl<E> ParamDeserializer<E> {
    fn new(str: String) -> Self {
        Self {
            str,
            phantom: std::marker::PhantomData,
        }
    }
}

macro_rules! forward_to_deserialize_from_str {
    ($($func:ident => $visit_func:ident,)*) => {
        $(
            fn $func<V: serde::de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
                visitor.$visit_func(self.str.parse().map_err(serde::de::Error::custom)?)
            }
        )*
    };
}

impl<'de, E: serde::de::Error> Deserializer<'de> for ParamDeserializer<E> {
    type Error = E;

    fn deserialize_bool<V: serde::de::Visitor<'de>>(
        mut self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.str.make_ascii_lowercase();
        visitor.visit_bool(self.str.parse().map_err(serde::de::Error::custom)?)
    }

    fn deserialize_option<V: serde::de::Visitor<'de>>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_some(self)
    }

    forward_to_deserialize_from_str! {
        deserialize_i8 => visit_i8,
        deserialize_i16 => visit_i16,
        deserialize_i32 => visit_i32,
        deserialize_i64 => visit_i64,
        deserialize_i128 => visit_i128,
        deserialize_u8 => visit_u8,
        deserialize_u16 => visit_u16,
        deserialize_u32 => visit_u32,
        deserialize_u64 => visit_u64,
        deserialize_u128 => visit_u128,
        deserialize_f32 => visit_f32,
        deserialize_f64 => visit_f64,
    }

    forward_to_deserialize_any! {
        char str string seq map struct identifier ignored_any bytes byte_buf unit unit_struct newtype_struct tuple tuple_struct enum
    }

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_string(self.str)
    }
}

impl<'de> Deserializer<'de> for &'de mut ASCOMParams {
    type Error = serde::de::value::Error;

    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        Err(serde::de::Error::custom(
            "ASCOMParams can be deserialized only into a struct",
        ))
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes byte_buf option unit unit_struct newtype_struct seq tuple tuple_struct map enum identifier ignored_any
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        struct MapAccess<'de> {
            fields: &'static [&'static str],
            params: &'de mut ASCOMParams,
        }

        impl<'de> serde::de::MapAccess<'de> for MapAccess<'de> {
            type Error = serde::de::value::Error;

            fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
            where
                K: serde::de::DeserializeSeed<'de>,
            {
                if let Some(&field) = self.fields.first() {
                    seed.deserialize(BorrowedStrDeserializer::new(field))
                        .map(Some)
                } else {
                    Ok(None)
                }
            }

            fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
            where
                V: serde::de::DeserializeSeed<'de>,
            {
                let (&field, fields) = self
                    .fields
                    .split_first()
                    .expect("field name must be available if we reached this state");
                self.fields = fields;
                let field: &CaseInsensitiveStr = field.as_ref();
                match self.params.0.remove(field) {
                    Some(value) => seed.deserialize(ParamDeserializer::new(value)),
                    None => seed.deserialize(UnitDeserializer::new()),
                }
            }
        }

        visitor.visit_map(MapAccess {
            fields,
            params: self,
        })
    }
}

impl ASCOMParams {
    pub fn try_as<T: DeserializeOwned>(mut self) -> Result<T, serde::de::value::Error> {
        let value = T::deserialize(&mut self)?;
        if !self.0.is_empty() {
            return Err(serde::de::Error::custom(format!(
                "Unexpected fields: {:?}",
                self.0.keys()
            )));
        }
        Ok(value)
    }
}

// #[derive(Deserialize)]
pub(crate) struct ASCOMRequest {
    // #[serde(flatten)]
    pub(crate) transaction: TransactionIds,
    // #[serde(flatten)]
    pub(crate) encoded_params: ASCOMParams,
}

// Work around infamous serde(flatten) deserialization issues by manually
// buffering all the params in a HashMap<String, String> and then using
// serde_plain + serde::de::value::MapDeserializer to decode specific
// subtypes in ASCOMParams::try_as.
impl<'de> Deserialize<'de> for ASCOMRequest {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let mut encoded_params = ASCOMParams::deserialize(deserializer)?;
        let transaction =
            TransactionIds::deserialize(&mut encoded_params).map_err(serde::de::Error::custom)?;
        Ok(Self {
            transaction,
            encoded_params,
        })
    }
}

#[derive(Serialize)]
pub(crate) struct ASCOMResponse {
    #[serde(flatten)]
    transaction: TransactionIds,
    #[serde(flatten, serialize_with = "serialize_result")]
    result: ASCOMResult<OpaqueResponse>,
}

impl IntoResponse for ASCOMResponse {
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}

fn serialize_result<R: Serialize, S: serde::Serializer>(
    value: &ASCOMResult<R>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match value {
        Ok(value) => value.serialize(serializer),
        Err(error) => error.serialize(serializer),
    }
}
