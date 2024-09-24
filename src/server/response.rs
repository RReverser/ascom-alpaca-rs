use super::{Error, ResponseTransaction, ResponseWithTransaction};
use crate::response::ValueResponse;
use crate::{ASCOMError, ASCOMErrorCode, ASCOMResult};
use axum::response::IntoResponse;
use axum::Json;
use http::StatusCode;
use serde::Serialize;

pub(crate) trait Response: Sized {
    fn into_axum(self, transaction: ResponseTransaction) -> impl IntoResponse;
}

impl<T: Serialize> Response for ValueResponse<T> {
    fn into_axum(self, transaction: ResponseTransaction) -> impl IntoResponse {
        Json(ResponseWithTransaction {
            transaction,
            response: self,
        })
    }
}

impl<T: Serialize> Response for ASCOMResult<T> {
    fn into_axum(self, transaction: ResponseTransaction) -> impl IntoResponse {
        #[derive(Serialize)]
        struct Repr<T> {
            #[serde(flatten)]
            error: ASCOMError,
            #[serde(rename = "Value")]
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
    }
}

impl<T> Response for super::Result<T>
where
    ASCOMResult<T>: Response,
{
    fn into_axum(self, transaction: ResponseTransaction) -> impl IntoResponse {
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

        ascom_result_or_err.map(|ascom_result| ascom_result.into_axum(transaction))
    }
}
