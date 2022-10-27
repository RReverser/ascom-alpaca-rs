use super::rpc::OpaqueResponse;
use crate::ASCOMResult;
use serde::de::value::MapDeserializer;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::sync::atomic::AtomicU32;

#[derive(Serialize, Deserialize)]
struct TransactionIds {
    #[serde(rename = "ClientID")]
    #[serde(skip_serializing)]
    #[allow(dead_code)]
    client_id: Option<u32>,
    #[serde(rename = "ClientTransactionID")]
    client_transaction_id: Option<u32>,
    #[serde(rename = "ServerTransactionID")]
    #[serde(skip_deserializing)]
    #[serde(default = "generate_server_transaction_id")]
    server_transaction_id: u32,
}

impl TransactionIds {
    fn span(&self) -> tracing::Span {
        tracing::debug_span!(
            "alpaca_transaction",
            client_id = self.client_id,
            client_transaction_id = self.client_transaction_id,
            server_transaction_id = self.server_transaction_id,
        )
    }
}

fn generate_server_transaction_id() -> u32 {
    static SERVER_TRANSACTION_ID: AtomicU32 = AtomicU32::new(0);
    SERVER_TRANSACTION_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

#[derive(Debug, Deserialize)]
#[serde(transparent)]
pub struct ASCOMParams(HashMap<String, String>);

struct PlainDeserializerWithSpecialBool<'de> {
    inner: serde_plain::Deserializer<'de>,
}

impl<'de> PlainDeserializerWithSpecialBool<'de> {
    fn new(s: &'de str) -> Self {
        Self {
            inner: serde_plain::Deserializer::new(s),
        }
    }
}

impl<'de> serde::de::IntoDeserializer<'de, serde_plain::Error>
    for PlainDeserializerWithSpecialBool<'de>
{
    type Deserializer = Self;

    fn into_deserializer(self) -> Self::Deserializer {
        self
    }
}

macro_rules! forward_to_inner_deserializer {
    ($(fn $method:ident ($($arg:ident: $arg_ty:ty),*);)*) => {$(
        fn $method<V: serde::de::Visitor<'de>>(self  $(, $arg: $arg_ty)*, visitor: V) -> Result<V::Value, Self::Error> {
            self.inner.$method($($arg,)* visitor)
        }
    )*};
}

impl<'de> Deserializer<'de> for PlainDeserializerWithSpecialBool<'de> {
    type Error = serde_plain::Error;

    fn deserialize_bool<V: serde::de::Visitor<'de>>(
        self,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        match <&str>::deserialize(self.inner)? {
            "True" | "true" => visitor.visit_bool(true),
            "False" | "false" => visitor.visit_bool(false),
            other => Err(serde::de::Error::unknown_variant(
                other,
                &["True", "False", "true", "false"],
            )),
        }
    }

    forward_to_inner_deserializer! {
        fn deserialize_any();
        fn deserialize_option();
        fn deserialize_seq();
        fn deserialize_map();
        fn deserialize_struct(name: &'static str, fields: &'static [&'static str]);
        fn deserialize_identifier();
        fn deserialize_ignored_any();
        fn deserialize_u8();
        fn deserialize_u16();
        fn deserialize_u32();
        fn deserialize_u64();
        fn deserialize_u128();
        fn deserialize_i8();
        fn deserialize_i16();
        fn deserialize_i32();
        fn deserialize_i64();
        fn deserialize_i128();
        fn deserialize_f32();
        fn deserialize_f64();
        fn deserialize_char();
        fn deserialize_str();
        fn deserialize_string();
        fn deserialize_bytes();
        fn deserialize_byte_buf();
        fn deserialize_unit();
        fn deserialize_unit_struct(name: &'static str);
        fn deserialize_newtype_struct(name: &'static str);
        fn deserialize_tuple(len: usize);
        fn deserialize_tuple_struct(name: &'static str, len: usize);
        fn deserialize_enum(name: &'static str, variants: &'static [&'static str]);
    }
}

impl ASCOMParams {
    pub fn try_as<T: DeserializeOwned>(&self) -> Result<T, serde_plain::Error> {
        let deserializer = MapDeserializer::new(
            self.0
                .iter()
                .map(|(k, v)| (k.as_str(), PlainDeserializerWithSpecialBool::new(v))),
        );

        T::deserialize(deserializer)
    }
}

// #[derive(Deserialize)]
pub(crate) struct ASCOMRequest {
    // #[serde(flatten)]
    transaction: TransactionIds,
    // #[serde(flatten)]
    encoded_params: ASCOMParams,
}

// Work around infamous serde(flatten) deserialization issues by manually
// buffering all theparams in a HashMap<String, String> and then using
// serde_plain + serde::de::value::MapDeserializer to decode specific
// subtypes in ASCOMParams::try_as.
impl<'de> Deserialize<'de> for ASCOMRequest {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let mut encoded_params = ASCOMParams::deserialize(deserializer)?;
        let transaction = encoded_params.try_as().map_err(serde::de::Error::custom)?;
        encoded_params.0.remove("ClientID");
        encoded_params.0.remove("ClientTransactionID");
        Ok(Self {
            transaction,
            encoded_params,
        })
    }
}

impl ASCOMRequest {
    pub(crate) fn respond_with<F: FnOnce(ASCOMParams) -> ASCOMResult<OpaqueResponse>>(
        self,
        f: F,
    ) -> ASCOMResponse {
        let span = self.transaction.span();
        let _span_enter = span.enter();

        ASCOMResponse {
            transaction: self.transaction,
            result: f(self.encoded_params),
        }
    }
}

#[derive(Serialize)]
pub(crate) struct ASCOMResponse {
    #[serde(flatten)]
    transaction: TransactionIds,
    #[serde(flatten, serialize_with = "serialize_result")]
    result: ASCOMResult<OpaqueResponse>,
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
