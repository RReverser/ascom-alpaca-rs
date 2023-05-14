use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use thiserror::Error;

/// Alpaca representation of an ASCOM error code.
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ASCOMErrorCode(u16);

impl TryFrom<u16> for ASCOMErrorCode {
    type Error = eyre::Error;

    /// Convert a raw error code into an `ASCOMErrorCode` if it's in the valid range.
    fn try_from(raw: u16) -> eyre::Result<Self> {
        let range = BASE..=MAX;
        eyre::ensure!(
            range.contains(&raw),
            "Error code {raw:#X} is out of valid range ({range:#X?})",
            raw = raw,
            range = range,
        );
        Ok(Self(raw))
    }
}

/// The starting value for error numbers.
const BASE: u16 = 0x400;
/// The starting value for driver-specific error numbers.
const DRIVER_BASE: u16 = 0x500;
/// The maximum value for error numbers.
const MAX: u16 = 0xFFF;

impl ASCOMErrorCode {
    /// Generate a driver-specific error code (supply code starting from `0`).
    ///
    /// This is intentionally limited to be usable only in `const` contexts
    /// so that you don't accidentally supply invalid values.
    ///
    /// # Example
    ///
    /// ```
    /// # use ascom_alpaca::{ASCOMError, ASCOMErrorCode};
    /// // Create constants for driver-specific error codes.
    /// const MY_ERROR_CODE: ASCOMErrorCode = ASCOMErrorCode::new_for_driver::<0>();
    /// const MY_OTHER_ERROR_CODE: ASCOMErrorCode = ASCOMErrorCode::new_for_driver::<1>();
    /// // ...somewhere later...
    /// let error = ASCOMError::new(MY_ERROR_CODE, "Something went wrong");
    /// ```
    pub const fn new_for_driver<const CODE: u16>() -> Self {
        let raw = match CODE.checked_add(DRIVER_BASE) {
            Some(raw) if raw <= MAX => raw,
            _ => panic!("Driver error code is too large"),
        };
        Self(raw)
    }

    /// Get the driver-specific error code.
    ///
    /// Returns `Ok` with `0`-based driver error code if this is a driver error.
    /// Returns `Err` with raw error code if not a driver error.
    pub const fn as_driver_error(self) -> Result<u16, u16> {
        if let Some(driver_code) = self.0.checked_sub(DRIVER_BASE) {
            Ok(driver_code)
        } else {
            Err(self.0)
        }
    }

    /// Get the raw error code.
    pub const fn raw(self) -> u16 {
        self.0
    }
}

/// ASCOM error.
#[derive(Debug, Clone, Serialize, Deserialize, Error)]
#[error("ASCOM error {code}: {message}")]
pub struct ASCOMError {
    /// Error number.
    #[serde(rename = "ErrorNumber")]
    pub code: ASCOMErrorCode,
    /// Error message.
    #[serde(rename = "ErrorMessage")]
    pub message: Cow<'static, str>,
}

impl ASCOMError {
    /// Create a new `ASCOMError` from given error code and a message.
    pub fn new(code: ASCOMErrorCode, message: impl std::fmt::Display) -> Self {
        Self {
            code,
            message: message.to_string().into(),
        }
    }
}

/// Result type for ASCOM methods.
pub type ASCOMResult<T = ()> = Result<T, ASCOMError>;

macro_rules! ascom_error_codes {
    ($(#[doc = $doc:literal] $name:ident = $value:literal,)*) => {
        impl ASCOMErrorCode {
            $(
                #[doc = $doc]
                pub const $name: Self = Self($value);
            )*
        }

        impl std::fmt::Debug for ASCOMErrorCode {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match *self {
                    $(
                        Self::$name => write!(f, "{}", stringify!($name)),
                    )*
                    _ => match self.as_driver_error() {
                        Ok(driver_code) => write!(f, "DRIVER_ERROR[{driver_code}]"),
                        Err(raw_code) => write!(f, "{raw_code:#X}"),
                    },
                }
            }
        }

        impl std::fmt::Display for ASCOMErrorCode {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                std::fmt::Debug::fmt(self, f)
            }
        }

        impl ASCOMError {
            $(
                #[doc = $doc]
                pub const $name: Self = Self {
                    code: ASCOMErrorCode::$name,
                    message: Cow::Borrowed($doc),
                };
            )*
        }
    };
}

ascom_error_codes! {
    #[doc = ""]
    OK = 0,
    #[doc = "The requested action is not implemented in this driver"]
    ACTION_NOT_IMPLEMENTED = 0x40C,
    #[doc = "The requested operation can not be undertaken at this time"]
    INVALID_OPERATION = 0x40B,
    #[doc = "Invalid value"]
    INVALID_VALUE = 0x401,
    #[doc = "The attempted operation is invalid because the mount is currently in a Parked state"]
    INVALID_WHILE_PARKED = 0x408,
    #[doc = "The attempted operation is invalid because the mount is currently in a Slaved state"]
    INVALID_WHILE_SLAVED = 0x409,
    #[doc = "The communications channel is not connected"]
    NOT_CONNECTED = 0x407,
    #[doc = "Property or method not implemented"]
    NOT_IMPLEMENTED = 0x400,
    #[doc = "The requested item is not present in the ASCOM cache"]
    NOT_IN_CACHE = 0x40D,
    #[doc = "Settings error"]
    SETTINGS = 0x40A,
    #[doc = "Unspecified error"]
    UNSPECIFIED = 0x4FF,
    #[doc = "A value has not been set"]
    VALUE_NOT_SET = 0x402,
}

impl ASCOMError {
    /// Generate a driver-specific error with the given code (0-based) and a message.
    pub fn driver_error<const CODE: u16>(message: impl std::fmt::Display) -> Self {
        Self {
            code: ASCOMErrorCode::new_for_driver::<CODE>(),
            message: message.to_string().into(),
        }
    }

    /// Create a new "invalid operation" error with the specified message.
    pub fn invalid_operation(message: impl std::fmt::Display) -> Self {
        Self::new(ASCOMErrorCode::INVALID_OPERATION, message)
    }

    /// Create a new "invalid value" error with the specified message.
    pub fn invalid_value(message: impl std::fmt::Display) -> Self {
        Self::new(ASCOMErrorCode::INVALID_VALUE, message)
    }

    /// Create a new error with unspecified error code and the given message.
    pub fn unspecified(message: impl std::fmt::Display) -> Self {
        Self::new(ASCOMErrorCode::UNSPECIFIED, message)
    }
}
