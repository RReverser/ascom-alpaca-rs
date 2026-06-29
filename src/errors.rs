use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::fmt::{self, Debug, Display, Formatter, UpperHex};
use std::ops::RangeInclusive;
use thiserror::Error;

/// The starting value for error numbers.
const BASE: u16 = 0x400;
/// The starting value for driver-specific error numbers.
const DRIVER_BASE: u16 = 0x500;
/// The maximum value for error numbers.
const MAX: u16 = 0xFFF;

/// The valid range of error numbers.
const RANGE: RangeInclusive<u16> = BASE..=MAX;

fn invalid_error_code(raw: impl UpperHex) -> eyre::Error {
    eyre::eyre!("Error code {raw:#X} is out of valid range ({RANGE:#X?})")
}

/// Alpaca representation of an ASCOM error code.
#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, derive_more::Display)]
#[display("{self:?}")]
#[serde(transparent)]
pub struct ASCOMErrorCode(u16);

impl TryFrom<i32> for ASCOMErrorCode {
    type Error = eyre::Error;

    /// Convert a raw error code into an `ASCOMErrorCode` if it's in the valid range.
    fn try_from(raw: i32) -> eyre::Result<Self> {
        if let Ok(raw) = u16::try_from(raw)
            && RANGE.contains(&raw)
        {
            Ok(Self(raw))
        } else {
            Err(invalid_error_code(raw))
        }
    }
}

impl TryFrom<u16> for ASCOMErrorCode {
    type Error = eyre::Error;

    /// Convert a raw error code into an `ASCOMErrorCode` if it's in the valid range.
    fn try_from(raw: u16) -> eyre::Result<Self> {
        if RANGE.contains(&raw) {
            Ok(Self(raw))
        } else {
            Err(invalid_error_code(raw))
        }
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
#[derive(Debug, Clone, Serialize, Error)]
#[error("ASCOM error {code}: {message}")]
pub struct ASCOMError {
    /// Error number.
    #[serde(rename = "ErrorNumber")]
    pub code: ASCOMErrorCode,
    /// Error message.
    #[serde(rename = "ErrorMessage")]
    pub message: Cow<'static, str>,
}

impl<'de> Deserialize<'de> for ASCOMError {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(rename_all = "PascalCase")]
        struct Repr {
            error_number: i32,
            error_message: Cow<'static, str>,
        }

        let Repr {
            error_number,
            error_message,
        } = Repr::deserialize(deserializer)?;

        // A zero error number is success; anything else goes through the shared out-of-range
        // handling so the JSON and `imagebytes` paths agree (issue #22).
        Ok(if error_number == 0 {
            Self {
                code: ASCOMErrorCode::OK,
                message: error_message,
            }
        } else {
            Self::new_with_unbounded_code(error_number, &error_message)
        })
    }
}

impl ASCOMError {
    /// Create a new `ASCOMError` from given error code and a message.
    pub fn new(code: ASCOMErrorCode, message: impl Display) -> Self {
        Self {
            code,
            message: format!("{message:#}").into(),
        }
    }

    /// Create an `ASCOMError` from a raw, non-zero error number that may fall outside Alpaca's
    /// valid range.
    ///
    /// Drivers occasionally report HRESULT-style codes (e.g. `-2147024882`) that don't fit.
    ///
    /// Instead of failing hard, which would lose the original error message (see issue #22),
    /// we surface those as `UNSPECIFIED` with the range error prepended to the main one.
    pub(crate) fn new_with_unbounded_code(raw_code: i32, message: &str) -> Self {
        match ASCOMErrorCode::try_from(raw_code) {
            Ok(code) => Self::new(code, message),
            Err(err) => Self::new(
                ASCOMErrorCode::UNSPECIFIED,
                format_args!("{err}: {message}"),
            ),
        }
    }
}

/// Result type for ASCOM methods.
pub type ASCOMResult<T = ()> = Result<T, ASCOMError>;

macro_rules! ascom_error_codes {
    ($(#[doc = $doc:literal] $vis:vis $name:ident = $value:literal,)*) => {
        impl ASCOMErrorCode {
            $(
                #[doc = $doc]
                $vis const $name: Self = Self($value);
            )*
        }

        impl Debug for ASCOMErrorCode {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
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

        #[expect(unused)]
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
    pub(crate) UNSPECIFIED = 0x4FF,
}

impl ASCOMError {
    /// Create a new "invalid operation" error with the specified message.
    pub fn invalid_operation(message: impl Display) -> Self {
        Self::new(ASCOMErrorCode::INVALID_OPERATION, message)
    }

    /// Create a new "invalid value" error with the specified message.
    pub fn invalid_value(message: impl Display) -> Self {
        Self::new(ASCOMErrorCode::INVALID_VALUE, message)
    }

    /// Create a new error with unspecified error code and the given message.
    #[cfg(feature = "client")]
    pub(crate) fn unspecified(message: impl Display) -> Self {
        Self::new(ASCOMErrorCode::UNSPECIFIED, message)
    }
}

#[cfg(test)]
mod tests {
    use super::{ASCOMError, ASCOMErrorCode};
    use std::assert_matches;

    // Devices sometimes return HRESULT-style error numbers that don't fit `u16`. These must
    // deserialize as `UNSPECIFIED` (keeping the message) rather than failing the response (#22).
    #[test]
    fn deserializes_out_of_range_error_number_as_unspecified() -> eyre::Result<()> {
        assert_matches!(
            serde_json::from_str(r#"{"ErrorNumber": -2147024882, "ErrorMessage": "Not enough storage"}"#)?,
            ASCOMError {
                code: ASCOMErrorCode::UNSPECIFIED,
                message,
            } if message.contains("out of valid range") && message.contains("Not enough storage")
        );

        Ok(())
    }

    // In-range codes (including success `0`, which is outside the `0x400..=0xFFF` range but is a
    // valid `u16`) must round-trip unchanged so the client can still detect `OK` responses.
    #[test]
    fn deserializes_in_range_error_numbers_unchanged() -> eyre::Result<()> {
        assert_matches!(
            serde_json::from_str(r#"{"ErrorNumber": 0, "ErrorMessage": ""}"#)?,
            ASCOMError {
                code: ASCOMErrorCode::OK,
                ..
            }
        );

        assert_matches!(
            serde_json::from_str(r#"{"ErrorNumber": 1025, "ErrorMessage": "bad"}"#)?,
            ASCOMError {
                code: ASCOMErrorCode::INVALID_VALUE,
                ..
            }
        );

        Ok(())
    }
}
