use super::{Error, ResponseTransaction, ResponseWithTransaction};
use crate::response::ValueResponse;
use crate::{ASCOMError, ASCOMResult};
use axum::http::StatusCode;
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

impl<T> Response for Result<T, Error>
where
    ASCOMResult<T>: Response,
{
    fn into_axum(self, transaction: ResponseTransaction) -> axum::response::Response {
        let ascom_result_or_err = match self {
            Ok(response) => Ok(Ok(response)),
            Err(Error::Ascom(err)) => Ok(Err(err)),
            Err(Error::BadRequest(err)) => Err((StatusCode::BAD_REQUEST, format!("{err:#}"))),
            Err(Error::NotFound(err)) => Err((StatusCode::NOT_FOUND, format!("{err:#}"))),
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
