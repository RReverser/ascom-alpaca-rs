use crate::server::ResponseWithTransaction;
use crate::{server, ASCOMError, ASCOMErrorCode, ASCOMResult};
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
#[serde(transparent)]
pub(crate) struct OpaqueResponse(serde_json::Map<String, serde_json::Value>);

impl OpaqueResponse {
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
                // all methods must return structs, `()` or use `ValueResponse` struct wrapper
                unreachable!("internal error: expected object or null, got {value:?}")
            }
        })
    }
}

pub(crate) trait Response: Sized {
    fn into_axum(self, transaction: crate::server::ResponseTransaction)
        -> axum::response::Response;
}

impl Response for OpaqueResponse {
    fn into_axum(self, transaction: server::ResponseTransaction) -> axum::response::Response {
        Json(ResponseWithTransaction {
            transaction,
            response: self,
        })
        .into_response()
    }
}

impl Response for ASCOMResult<OpaqueResponse> {
    fn into_axum(self, transaction: server::ResponseTransaction) -> axum::response::Response {
        #[derive(Serialize)]
        struct Repr {
            #[serde(flatten)]
            error: ASCOMError,
            #[serde(flatten)]
            value: OpaqueResponse,
        }

        Json(ResponseWithTransaction {
            transaction,
            response: match self {
                Ok(value) => Repr {
                    error: ASCOMError::new(ASCOMErrorCode(0), ""),
                    value,
                },
                Err(error) => {
                    tracing::error!(%error, "Alpaca method returned an error");

                    Repr {
                        error,
                        value: OpaqueResponse::default(),
                    }
                }
            },
        })
        .into_response()
    }
}
