use super::{ResponseTransaction, ResponseWithTransaction};
use crate::{ASCOMError, ASCOMResult};
use axum::response::IntoResponse;
use axum::Json;
use serde::Serialize;

pub(crate) trait Response: Sized {
    fn into_axum(self, transaction: ResponseTransaction) -> axum::response::Response;
}

impl<T: Serialize> Response for ASCOMResult<T> {
    fn into_axum(self, transaction: ResponseTransaction) -> axum::response::Response {
        #[derive(Serialize)]
        struct Repr<T> {
            #[serde(flatten)]
            error: ASCOMError,
            #[serde(flatten)]
            #[serde(skip_serializing_if = "Option::is_none")]
            value: Option<T>,
        }

        Json(ResponseWithTransaction {
            transaction,
            response: match self {
                Ok(value) => Repr {
                    error: ASCOMError::OK,
                    value: Some(value),
                },
                Err(error) => {
                    tracing::error!(%error, "Alpaca method returned an error");
                    Repr { error, value: None }
                }
            },
        })
        .into_response()
    }
}
