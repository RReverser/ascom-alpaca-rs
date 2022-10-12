use super::rpc::OpaqueResponse;
use crate::{ASCOMError, ASCOMResult};
use serde::{Deserialize, Serialize};
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
        tracing::info_span!(
            "Alpaca transaction",
            client_id = self.client_id,
            client_transaction_id = self.client_transaction_id,
            server_transaction_id = self.server_transaction_id
        )
    }
}

fn generate_server_transaction_id() -> u32 {
    static SERVER_TRANSACTION_ID: AtomicU32 = AtomicU32::new(0);
    SERVER_TRANSACTION_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

// #[derive(Deserialize)]
struct ASCOMRequest {
    // #[serde(flatten)]
    transaction: TransactionIds,
    // #[serde(flatten)]
    encoded_params: String,
}

impl ASCOMRequest {
    /// This awkward machinery is to accomodate for the fact that the serde(flatten)
    /// breaks all deserialization because it collects data into an internal representation
    /// first and then can't recover other types from string values stored from the query string.
    ///
    /// See [nox/serde_urlencoded#33](https://github.com/nox/serde_urlencoded/issues/33).
    fn from_encoded_params(encoded_params: &str) -> Result<Self, serde_urlencoded::de::Error> {
        let mut transaction_params = form_urlencoded::Serializer::new(String::new());
        let mut request_params = form_urlencoded::Serializer::new(String::new());

        for (key, value) in form_urlencoded::parse(encoded_params.as_bytes()) {
            match key.as_ref() {
                "ClientID" | "ClientTransactionID" => {
                    transaction_params.append_pair(&key, &value);
                }
                _ => {
                    request_params.append_pair(&key, &value);
                }
            }
        }

        Ok(ASCOMRequest {
            transaction: serde_urlencoded::from_str(&transaction_params.finish())?,
            encoded_params: request_params.finish(),
        })
    }
}

#[derive(Serialize)]
struct ASCOMResponse {
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

pub fn respond_with(
    params: &str,
    handler: impl FnOnce(&str) -> Result<OpaqueResponse, ASCOMError>,
) -> Result<impl Serialize, impl serde::de::Error> {
    ASCOMRequest::from_encoded_params(params).map(move |request| {
        let _span = request.transaction.span();

        ASCOMResponse {
            transaction: request.transaction,
            result: handler(&request.encoded_params),
        }
    })
}
