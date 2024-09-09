use super::{Error, ResponseTransaction, ResponseWithTransaction};
use crate::response::ValueResponse;
use crate::{ASCOMError, ASCOMErrorCode, ASCOMResult};
use axum::response::IntoResponse;
use axum::Json;
use http::StatusCode;
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
                    if error.code == ASCOMErrorCode::NOT_IMPLEMENTED {
                        tracing::warn!("Alpaca method is not implemented");
                    } else {
                        tracing::error!(%error, "Alpaca method returned an error");
                    }
                    Repr { error, value: None }
                }
            },
        })
        .into_response()
    }
}

impl<T> Response for super::Result<T>
where
    ASCOMResult<T>: Response,
{
    fn into_axum(self, transaction: ResponseTransaction) -> axum::response::Response {
        let ascom_result_or_err = match self {
            Ok(response) => Ok(Ok(response)),
            Err(Error::Ascom(err)) => Ok(Err(err)),
            Err(err @ (Error::MissingParameter { .. } | Error::BadParameter { .. })) => {
                Err((StatusCode::BAD_REQUEST, err.to_string()))
            }
            Err(err @ (Error::UnknownDeviceIndex { .. } | Error::UnknownAction { .. })) => {
                Err((StatusCode::NOT_FOUND, err.to_string()))
            }
        };

        match ascom_result_or_err {
            Ok(ascom_result) => ascom_result.into_axum(transaction),
            Err(err) => err.into_response(),
        }
    }
}

impl<T: Serialize> Response for ValueResponse<T> {
    fn into_axum(self, transaction: ResponseTransaction) -> axum::response::Response {
        Json(ResponseWithTransaction {
            transaction,
            response: self,
        })
        .into_response()
    }
}
