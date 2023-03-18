use crate::{ASCOMError, ASCOMErrorCode, ASCOMResult};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
#[cfg(feature = "client")]
use {crate::client, bytes::Bytes, mime::Mime};
#[cfg(feature = "server")]
use {crate::server, axum::response::IntoResponse};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(transparent)]
pub(crate) struct OpaqueResponse(pub(crate) serde_json::Map<String, serde_json::Value>);

impl OpaqueResponse {
    #[cfg(feature = "server")]
    pub(crate) fn new<T: Debug + Serialize>(value: T) -> Self {
        let json = serde_json::to_value(&value).unwrap_or_else(|err| {
            // This should never happen, but if it does, log and return the error.
            // This simplifies error handling for this rare case without having to panic!.
            tracing::error!(?value, %err, "Serialization failure");
            serde_json::to_value(ASCOMError {
                code: ASCOMErrorCode::UNSPECIFIED,
                message: format!("Failed to serialize {value:#?}: {err}").into(),
            })
            .expect("ASCOMError can never fail to serialize")
        });

        Self(match json {
            serde_json::Value::Object(map) => map,
            serde_json::Value::Null => serde_json::Map::new(),
            value => {
                // Wrap into IntResponse / BoolResponse / ..., aka {"value": ...}
                std::iter::once(("Value".to_owned(), value)).collect()
            }
        })
    }

    #[cfg(feature = "client")]
    pub(crate) fn try_as<T: serde::de::DeserializeOwned>(mut self) -> serde_json::Result<T> {
        serde_json::from_value(if self.0.contains_key("Value") {
            #[allow(clippy::unwrap_used)]
            self.0.remove("Value").unwrap()
        } else {
            serde_json::Value::Object(self.0)
        })
    }
}

pub(crate) trait Response: Sized {
    #[cfg(feature = "server")]
    fn into_axum(self, transaction: crate::server::ResponseTransaction)
        -> axum::response::Response;

    #[cfg(feature = "client")]
    fn prepare_reqwest(request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        request
    }

    #[cfg(feature = "client")]
    fn from_reqwest(
        mime_type: Mime,
        bytes: Bytes,
    ) -> anyhow::Result<crate::client::ResponseWithTransaction<Self>>;
}

impl Response for OpaqueResponse {
    #[cfg(feature = "server")]
    fn into_axum(
        self,
        transaction: crate::server::ResponseTransaction,
    ) -> axum::response::Response {
        axum::response::Json(crate::server::ResponseWithTransaction {
            transaction,
            response: self,
        })
        .into_response()
    }

    #[cfg(feature = "client")]
    fn from_reqwest(
        mime_type: Mime,
        bytes: Bytes,
    ) -> anyhow::Result<crate::client::ResponseWithTransaction<Self>> {
        anyhow::ensure!(
            mime_type.essence_str() == mime::APPLICATION_JSON.as_ref(),
            "Expected JSON response, got {mime_type}"
        );
        match mime_type.get_param(mime::CHARSET) {
            Some(mime::UTF_8) | None => {}
            Some(charset) => anyhow::bail!("Unsupported charset {charset}"),
        };

        serde_json::from_slice::<Self>(&bytes)?.try_into()
    }
}

impl Response for ASCOMResult<OpaqueResponse> {
    #[cfg(feature = "server")]
    fn into_axum(self, transaction: server::ResponseTransaction) -> axum::response::Response {
        match self {
            Ok(mut res) => {
                res.0
                    .extend(OpaqueResponse::new(ASCOMError::new(ASCOMErrorCode(0), "")).0);
                res
            }
            Err(err) => {
                tracing::error!(%err, "Alpaca method returned an error");
                OpaqueResponse::new(err)
            }
        }
        .into_axum(transaction)
    }

    #[cfg(feature = "client")]
    fn from_reqwest(
        mime_type: Mime,
        bytes: Bytes,
    ) -> anyhow::Result<client::ResponseWithTransaction<Self>> {
        Ok(
            OpaqueResponse::from_reqwest(mime_type, bytes)?.map(|response| {
                match response.0.get("ErrorNumber") {
                    Some(error_number) if error_number != 0_i32 => {
                        Err(response.try_as::<ASCOMError>().unwrap_or_else(|err| {
                            ASCOMError::new(
                                ASCOMErrorCode::UNSPECIFIED,
                                format!(
                                    "Server returned an error but it couldn't be parsed: {err}"
                                ),
                            )
                        }))
                    }
                    _ => Ok(response),
                }
            }),
        )
    }
}
