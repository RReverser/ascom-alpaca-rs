use crate::api::DeviceType;
use crate::ASCOMError;
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

pub(crate) type Result<T> = std::result::Result<T, Error>;
