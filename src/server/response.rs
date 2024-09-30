use super::{Error, ResponseWithTransaction};
use crate::response::ValueResponse;
use crate::{ASCOMError, ASCOMErrorCode, ASCOMResult};
use axum::response::{IntoResponse, Response};
use axum::Json;
use http::StatusCode;
use serde::Serialize;

impl<T: Serialize> IntoResponse for ResponseWithTransaction<ValueResponse<T>> {
    fn into_response(self) -> Response {
        Json(self).into_response()
    }
}

impl<T: Serialize> IntoResponse for ResponseWithTransaction<ASCOMResult<T>> {
    fn into_response(self) -> Response {
        #[derive(Serialize)]
        struct Repr<T> {
            #[serde(flatten)]
            error: ASCOMError,
            #[serde(rename = "Value")]
            #[serde(skip_serializing_if = "Option::is_none")]
            value: Option<T>,
        }

        Json(ResponseWithTransaction {
            transaction: self.transaction,
            response: match self.response {
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

impl<T> IntoResponse for ResponseWithTransaction<super::Result<T>>
where
    ResponseWithTransaction<ASCOMResult<T>>: IntoResponse,
{
    fn into_response(self) -> Response {
        match self.response {
            Ok(response) => Ok(Ok(response)),
            Err(Error::Ascom(err)) => Ok(Err(err)),
            Err(err @ (Error::MissingParameter { .. } | Error::BadParameter { .. })) => {
                Err((StatusCode::BAD_REQUEST, err.to_string()))
            }
            Err(err @ (Error::UnknownDeviceIndex { .. } | Error::UnknownAction { .. })) => {
                Err((StatusCode::NOT_FOUND, err.to_string()))
            }
        }
        .map(|ascom_result| ResponseWithTransaction {
            transaction: self.transaction,
            response: ascom_result,
        })
        .into_response()
    }
}
