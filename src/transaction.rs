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

impl ASCOMParams {
    pub fn try_as<T: DeserializeOwned>(&self) -> Result<T, serde_plain::Error> {
        struct Plain<'de>(&'de str);

        impl<'de> serde::de::IntoDeserializer<'de, serde_plain::Error> for Plain<'de> {
            type Deserializer = serde_plain::Deserializer<'de>;

            fn into_deserializer(self) -> Self::Deserializer {
                serde_plain::Deserializer::new(self.0)
            }
        }

        let deserializer = MapDeserializer::new(self.0.iter().map(|(k, v)| (k.as_str(), Plain(v))));

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
        let encoded_params = ASCOMParams::deserialize(deserializer)?;
        Ok(Self {
            transaction: encoded_params.try_as().map_err(serde::de::Error::custom)?,
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
