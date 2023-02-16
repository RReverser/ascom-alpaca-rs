use super::rpc::OpaqueResponse;
use crate::rpc::ASCOMParam;
use anyhow::Context;
use async_trait::async_trait;
use axum::body::HttpBody;
use axum::extract::{FromRequest, RequestParts};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{BoxError, Form, Json};
use indexmap::IndexMap;
use serde::{Deserialize, Deserializer, Serialize};
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
    #[serde(skip_serializing_if = "Option::is_none")]
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

    pub(crate) const fn make_response(self, result: OpaqueResponse) -> ASCOMResponse {
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
pub(crate) struct CaseInsensitiveStr(str);

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

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub(crate) struct ASCOMParams(IndexMap<Box<CaseInsensitiveStr>, String>);

impl ASCOMParams {
    pub(crate) fn maybe_extract<T: ASCOMParam>(&mut self, name: &str) -> anyhow::Result<Option<T>> {
        self.0
            .remove::<CaseInsensitiveStr>(name.as_ref())
            .map(|value| {
                T::from_string(value).with_context(|| format!("Invalid value for parameter {name}"))
            })
            .transpose()
    }

    pub(crate) fn extract<T: ASCOMParam>(&mut self, name: &str) -> anyhow::Result<T> {
        self.maybe_extract(name)?
            .ok_or_else(|| anyhow::anyhow!("Missing parameter {name}"))
    }

    pub(crate) fn insert<T: ASCOMParam>(&mut self, name: &str, value: T) {
        let prev_value = self.0.insert(Box::<str>::from(name).into(), value.to_string());
        debug_assert!(prev_value.is_none());
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
        let transaction = TransactionIds {
            client_id: encoded_params
                .maybe_extract("ClientID")
                .map_err(serde::de::Error::custom)?,
            client_transaction_id: encoded_params
                .maybe_extract("ClientTransactionID")
                .map_err(serde::de::Error::custom)?,
            server_transaction_id: generate_server_transaction_id(),
        };
        Ok(Self {
            transaction,
            encoded_params,
        })
    }
}

#[async_trait]
impl<B> FromRequest<B> for ASCOMRequest
where
    B: HttpBody + Send,
    B::Data: Send,
    B::Error: Into<BoxError>,
{
    type Rejection = Response;

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        match Form::<Self>::from_request(req).await {
            Ok(Form(request)) => Ok(request),
            Err(err) => {
                let mut err = err.into_response();
                *err.status_mut() = StatusCode::BAD_REQUEST;
                Err(err)
            }
        }
    }
}

#[derive(Serialize)]
pub(crate) struct ASCOMResponse {
    #[serde(flatten)]
    transaction: TransactionIds,
    result: OpaqueResponse,
}

impl IntoResponse for ASCOMResponse {
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}
