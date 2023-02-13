use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use thiserror::Error;

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(transparent)]
pub struct ASCOMErrorCode(pub u16);

/// The starting value for driver-specific error numbers.
const DRIVER_BASE: u16 = 0x500;
/// The maximum value for driver-specific error numbers.
const DRIVER_MAX: u16 = 0xFFF;

impl ASCOMErrorCode {
    /// Generate a driver-specific error code.
    pub const fn new_for_driver(code: u16) -> Self {
        assert!(
            code <= DRIVER_MAX - DRIVER_BASE,
            "Driver error code out of range"
        );
        Self(DRIVER_BASE + code)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Error)]
#[error("ASCOM error {code}: {message}")]
pub struct ASCOMError {
    #[serde(rename = "ErrorNumber")]
    pub code: ASCOMErrorCode,
    #[serde(rename = "ErrorMessage")]
    pub message: Cow<'static, str>,
}

impl ASCOMError {
    pub fn new(code: ASCOMErrorCode, message: impl Into<Cow<'static, str>>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

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
          Self(code @ DRIVER_BASE..=DRIVER_MAX) => write!(f, "DRIVER_ERROR[{}]", code - DRIVER_BASE),
          Self(code) => write!(f, "{:#X}", code),
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
          message: Cow::Borrowed(stringify!($doc)),
        };
      )*
    }
  };
}

ascom_error_codes! {
  #[doc = "The requested action is not implemented in this driver."]
  ACTION_NOT_IMPLEMENTED = 0x40C,
  #[doc = "The requested operation can not be undertaken at this time."]
  INVALID_OPERATION = 0x40B,
  #[doc = "Invalid value."]
  INVALID_VALUE = 0x401,
  #[doc = "The attempted operation is invalid because the mount is currently in a Parked state."]
  INVALID_WHILE_PARKED = 0x408,
  #[doc = "The attempted operation is invalid because the mount is currently in a Slaved state."]
  INVALID_WHILE_SLAVED = 0x409,
  #[doc = "The communications channel is not connected."]
  NOT_CONNECTED = 0x407,
  #[doc = "Property or method not implemented."]
  NOT_IMPLEMENTED = 0x400,
  #[doc = "The requested item is not present in the ASCOM cache."]
  NOT_IN_CACHE = 0x40D,
  #[doc = "Settings error."]
  SETTINGS = 0x40A,
  #[doc = "'catch-all' error code used when nothing else was specified."]
  UNSPECIFIED = 0x4FF,
  #[doc = "A value has not been set."]
  VALUE_NOT_SET = 0x402,
}
