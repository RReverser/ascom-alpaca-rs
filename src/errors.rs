use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::ops::RangeInclusive;
use thiserror::Error;

/// The starting value for error numbers.
const BASE: u16 = 0x400;
/// The starting value for driver-specific error numbers.
const DRIVER_BASE: u16 = 0x500;
/// The maximum value for error numbers.
const MAX: u16 = 0xFFF;

/// Alpaca representation of an ASCOM error code.
#[derive(Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ASCOMErrorCode(u16);

impl TryFrom<u16> for ASCOMErrorCode {
    type Error = eyre::Error;

    /// Convert a raw error code into an `ASCOMErrorCode` if it's in the valid range.
    fn try_from(raw: u16) -> eyre::Result<Self> {
        const RANGE: RangeInclusive<u16> = BASE..=MAX;

        eyre::ensure!(
            RANGE.contains(&raw),
            "Error code {raw:#X} is out of valid range ({RANGE:#X?})"
        );
        Ok(Self(raw))
    }
}

impl ASCOMErrorCode {
    /// Generate ASCOM error code from a zero-based driver error code.
    ///
    /// Will panic if the driver error code is larger than the maximum allowed (2815).
    ///
    /// You'll typically want to define an enum for your driver errors and use this in a single
    /// place - in the [`From`] conversion from your driver error type to the [`ASCOMError`].
    ///
    /// # Example
    ///
    /// ```
    /// use ascom_alpaca::{ASCOMError, ASCOMErrorCode};
    /// use thiserror::Error;
    ///
    /// #[derive(Debug, Error)]
    /// pub enum MyDriverError {
    ///     #[error("Port communication error: {0}")]
    ///     PortError(#[from] std::io::Error),
    ///     #[error("Initialization error: {0}")]
    ///     InitializationError(String),
    /// }
    ///
    /// // this allows you to then use `my_driver.method()?` when implementing Alpaca traits
    /// // and it will convert your driver error to an ASCOM error automatically
    /// impl From<MyDriverError> for ASCOMError {
    ///     fn from(error: MyDriverError) -> Self {
    ///         ASCOMError::new(
    ///             ASCOMErrorCode::new_for_driver(match error {
    ///                 MyDriverError::PortError(_) => 0,
    ///                 MyDriverError::InitializationError(_) => 1,
    ///             }),
    ///             error,
    ///         )
    ///     }
    /// }
    /// ```
    pub const fn new_for_driver(driver_code: u16) -> Self {
        const DRIVER_MAX: u16 = MAX - DRIVER_BASE;

        assert!(driver_code <= DRIVER_MAX, "Driver error code is too large");

        Self(driver_code + DRIVER_BASE)
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

pub(crate) trait ASCOMResultOk {
    type Ok;
}

impl<T> ASCOMResultOk for ASCOMResult<T> {
    type Ok = T;
}

macro_rules! ascom_error_codes {
    ($(#[doc = $doc:literal] $vis:vis $name:ident = $value:literal,)*) => {
        impl ASCOMErrorCode {
            $(
                #[doc = $doc]
                $vis const $name: Self = Self($value);
            )*
        }

        impl std::fmt::Debug for ASCOMErrorCode {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match *self {
                    $(
                        Self::$name => f.write_str(stringify!($name)),
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

        #[allow(unused)]
        impl ASCOMError {
            $(
                #[doc = $doc]
                $vis const $name: Self = Self {
                    code: ASCOMErrorCode::$name,
                    message: Cow::Borrowed(ascom_error_codes!(@msg $name $doc)),
                };
            )*
        }
    };

    (@msg OK $doc:literal) => ("");
    (@msg $name:ident $doc:literal) => ($doc.trim_ascii());
}

ascom_error_codes! {
    /// Success.
    pub OK = 0,

    // Well-known Alpaca error codes as per the specification.

    /// Property or method not implemented.
    pub NOT_IMPLEMENTED = 0x400,
    /// Invalid value.
    pub INVALID_VALUE = 0x401,
    /// A value has not been set.
    pub VALUE_NOT_SET = 0x402,
    /// The communications channel is not connected.
    pub NOT_CONNECTED = 0x407,
    /// The attempted operation is invalid because the mount is currently in a Parked state.
    pub INVALID_WHILE_PARKED = 0x408,
    /// The attempted operation is invalid because the mount is currently in a Slaved state.
    pub INVALID_WHILE_SLAVED = 0x409,
    /// The requested operation can not be undertaken at this time.
    pub INVALID_OPERATION = 0x40B,
    /// The requested action is not implemented in this driver.
    pub ACTION_NOT_IMPLEMENTED = 0x40C,
    /// In-progress asynchronous operation has been cancelled.
    pub OPERATION_CANCELLED = 0x40D,

    // Extra codes for internal use only.

    /// Reserved 'catch-all' error code (0x4FF) used when nothing else was specified.
    UNSPECIFIED = 0x4FF,
}

impl ASCOMError {
    /// Create a new "invalid operation" error with the specified message.
    pub fn invalid_operation(message: impl std::fmt::Display) -> Self {
        Self::new(ASCOMErrorCode::INVALID_OPERATION, message)
    }

    /// Create a new "invalid value" error with the specified message.
    pub fn invalid_value(message: impl std::fmt::Display) -> Self {
        Self::new(ASCOMErrorCode::INVALID_VALUE, message)
    }

    /// Create a new error with unspecified error code and the given message.
    #[cfg(feature = "client")]
    pub(crate) fn unspecified(message: impl std::fmt::Display) -> Self {
        Self::new(ASCOMErrorCode::UNSPECIFIED, message)
    }
}
