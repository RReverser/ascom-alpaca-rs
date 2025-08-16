use crate::api::DeviceType;
use crate::ASCOMError;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use thiserror::Error;

#[derive(Error, Debug)]
pub(crate) enum Error {
    #[error("Device {ty}[{index}] not found")]
    UnknownDeviceIndex { ty: DeviceType, index: usize },
    #[error("Unknown action {device_type}::{action}")]
    UnknownAction {
        device_type: DeviceType,
        action: String,
    },
    #[error("Missing parameter {name:?}")]
    MissingParameter { name: &'static str },
    #[error("Couldn't parse parameter {name:?}: {err:#}")]
    BadParameter {
        name: &'static str,
        #[source]
        err: serde_plain::Error,
    },
    #[error(transparent)]
    Ascom(#[from] ASCOMError),
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let code = match self {
            Self::UnknownDeviceIndex { .. } | Self::UnknownAction { .. } => StatusCode::NOT_FOUND,
            Self::MissingParameter { .. } | Self::BadParameter { .. } => StatusCode::BAD_REQUEST,
            Self::Ascom(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (code, format!("{self:#}")).into_response()
    }
}

pub(crate) type Result<T> = std::result::Result<T, Error>;
