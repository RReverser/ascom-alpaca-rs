use super::Error;
use super::case_insensitive_str::CaseInsensitiveStr;
use axum::Form;
use axum::extract::{FromRequest, Request};
use axum::response::{IntoResponse, Response};
use http::{Method, StatusCode};
use indexmap::IndexMap;
use serde::Deserialize;
use serde::de::{DeserializeOwned, Deserializer, Error as _, Visitor};
use std::fmt::{self, Debug};
use std::hash::Hash;

/// Error type for ALPACA parameter parsing that distinguishes parse errors from range errors.
#[derive(Debug)]
enum AlpacaParseError {
    /// Invalid format (not a valid integer/bool/etc) -> `BadParameter` (HTTP 400)
    BadFormat(String),
    /// Valid integer but out of range for target type -> `INVALID_VALUE` (ASCOM error)
    OutOfRange {
        value: i64,
        target_type: &'static str,
    },
}

impl fmt::Display for AlpacaParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadFormat(msg) => write!(f, "{msg}"),
            Self::OutOfRange { value, target_type } => {
                write!(f, "value {value} is out of range for {target_type}")
            }
        }
    }
}

impl std::error::Error for AlpacaParseError {}

impl serde::de::Error for AlpacaParseError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Self::BadFormat(msg.to_string())
    }
}

/// Custom serde Deserializer for ALPACA parameters.
///
/// Integers are parsed as i64 first, then converted to the target type.
/// This allows distinguishing parse errors (`BadParameter`) from range errors
/// (`INVALID_VALUE`), and supports both i32 device parameters and uint32
/// transaction/identity parameters (`ClientID`, `ClientTransactionID`).
struct AlpacaDeserializer {
    value: String,
}

impl AlpacaDeserializer {
    /// Parse an integer: parse as i64 first, then convert to the target type.
    fn parse_integer<T: TryFrom<i64>>(&self) -> Result<T, AlpacaParseError> {
        let i64_value: i64 = self.value.trim().parse().map_err(|_parse_err| {
            AlpacaParseError::BadFormat(format!("invalid integer: {}", self.value))
        })?;

        T::try_from(i64_value).map_err(|_range_err| AlpacaParseError::OutOfRange {
            value: i64_value,
            target_type: std::any::type_name::<T>(),
        })
    }
}

impl<'de> Deserializer<'de> for AlpacaDeserializer {
    type Error = AlpacaParseError;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_string(self.value)
    }

    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.value.trim().to_ascii_lowercase().as_str() {
            "true" => visitor.visit_bool(true),
            "false" => visitor.visit_bool(false),
            _ => Err(AlpacaParseError::BadFormat(format!(
                "invalid boolean: {}",
                self.value
            ))),
        }
    }

    fn deserialize_i8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_i8(self.parse_integer()?)
    }

    fn deserialize_i16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_i16(self.parse_integer()?)
    }

    fn deserialize_i32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_i32(self.parse_integer()?)
    }

    fn deserialize_i64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_i64(self.parse_integer()?)
    }

    fn deserialize_u8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_u8(self.parse_integer()?)
    }

    fn deserialize_u16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_u16(self.parse_integer()?)
    }

    fn deserialize_u32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_u32(self.parse_integer()?)
    }

    fn deserialize_u64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_u64(self.parse_integer()?)
    }

    fn deserialize_f32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let value: f32 = self.value.trim().parse().map_err(|_parse_err| {
            AlpacaParseError::BadFormat(format!("invalid float: {}", self.value))
        })?;
        visitor.visit_f32(value)
    }

    fn deserialize_f64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let value: f64 = self.value.trim().parse().map_err(|_parse_err| {
            AlpacaParseError::BadFormat(format!("invalid float: {}", self.value))
        })?;
        visitor.visit_f64(value)
    }

    fn deserialize_char<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let mut chars = self.value.chars();
        match (chars.next(), chars.next()) {
            (Some(c), None) => visitor.visit_char(c),
            _ => Err(AlpacaParseError::BadFormat(format!(
                "expected single character: {}",
                self.value
            ))),
        }
    }

    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_bytes(self.value.as_bytes())
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_byte_buf(self.value.into_bytes())
    }

    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_string(self.value)
    }

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_string(self.value)
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_enum(serde::de::value::StringDeserializer::new(self.value))
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_string(self.value)
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_string(self.value)
    }

    fn deserialize_option<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Self::Error> {
        Err(AlpacaParseError::BadFormat(
            "Option types are not supported as ALPACA parameters".to_owned(),
        ))
    }

    fn deserialize_unit<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Self::Error> {
        Err(AlpacaParseError::BadFormat(
            "unit type is not supported as ALPACA parameter".to_owned(),
        ))
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _visitor: V,
    ) -> Result<V::Value, Self::Error> {
        Err(AlpacaParseError::BadFormat(
            "unit struct is not supported as ALPACA parameter".to_owned(),
        ))
    }

    fn deserialize_seq<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Self::Error> {
        Err(AlpacaParseError::BadFormat(
            "sequences are not supported as ALPACA parameters".to_owned(),
        ))
    }

    fn deserialize_tuple<V: Visitor<'de>>(
        self,
        _len: usize,
        _visitor: V,
    ) -> Result<V::Value, Self::Error> {
        Err(AlpacaParseError::BadFormat(
            "tuples are not supported as ALPACA parameters".to_owned(),
        ))
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _len: usize,
        _visitor: V,
    ) -> Result<V::Value, Self::Error> {
        Err(AlpacaParseError::BadFormat(
            "tuple structs are not supported as ALPACA parameters".to_owned(),
        ))
    }

    fn deserialize_map<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Self::Error> {
        Err(AlpacaParseError::BadFormat(
            "maps are not supported as ALPACA parameters".to_owned(),
        ))
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Self::Error> {
        Err(AlpacaParseError::BadFormat(
            "structs are not supported as ALPACA parameters".to_owned(),
        ))
    }
}

#[derive(Deserialize, derive_more::Debug)]
#[debug("{_0:?}")]
#[serde(transparent)]
#[serde(bound(deserialize = "Box<ParamStr>: DeserializeOwned + Hash + Eq"))]
pub(crate) struct OpaqueParams<ParamStr: ?Sized>(IndexMap<Box<ParamStr>, String>);

#[derive(Debug)]
pub(crate) enum ActionParams {
    Get(OpaqueParams<CaseInsensitiveStr>),
    Put(OpaqueParams<str>),
}

impl<ParamStr: ?Sized + Hash + Eq + Debug> OpaqueParams<ParamStr>
where
    str: AsRef<ParamStr>,
{
    pub(crate) fn maybe_extract<T: DeserializeOwned>(
        &mut self,
        name: &'static str,
    ) -> super::Result<Option<T>> {
        self.0
            .swap_remove(name.as_ref())
            .map(|value| {
                T::deserialize(AlpacaDeserializer { value }).map_err(|err| match err {
                    AlpacaParseError::OutOfRange { value, target_type } => {
                        Error::ParameterOutOfRange {
                            name,
                            value,
                            target_type,
                        }
                    }
                    AlpacaParseError::BadFormat(msg) => Error::BadParameter {
                        name,
                        err: serde_plain::Error::custom(msg),
                    },
                })
            })
            .transpose()
    }

    pub(crate) fn extract<T: DeserializeOwned>(&mut self, name: &'static str) -> super::Result<T> {
        self.maybe_extract(name)?
            .ok_or(Error::MissingParameter { name })
    }

    pub(crate) fn finish_extraction(self) {
        if !self.0.is_empty() {
            tracing::warn!("Unused parameters: {:?}", self.0.keys());
        }
    }
}

impl ActionParams {
    pub(crate) fn finish_extraction(self) {
        match self {
            Self::Get(params) => params.finish_extraction(),
            Self::Put(params) => params.finish_extraction(),
        }
    }
}

impl<S: Send + Sync> FromRequest<S> for ActionParams {
    type Rejection = Response;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        match *req.method() {
            Method::GET => Ok(Self::Get(
                Form::from_request(req, state)
                    .await
                    .map_err(IntoResponse::into_response)?
                    .0,
            )),
            Method::PUT => Ok(Self::Put(
                Form::from_request(req, state)
                    .await
                    .map_err(IntoResponse::into_response)?
                    .0,
            )),
            _ => Err((StatusCode::METHOD_NOT_ALLOWED, "Method not allowed").into_response()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse<T: DeserializeOwned>(value: &str) -> super::super::Result<T> {
        let deser = AlpacaDeserializer {
            value: value.to_owned(),
        };
        T::deserialize(deser).map_err(|err| match err {
            AlpacaParseError::OutOfRange { value, target_type } => Error::ParameterOutOfRange {
                name: "test",
                value,
                target_type,
            },
            AlpacaParseError::BadFormat(msg) => Error::BadParameter {
                name: "test",
                err: serde_plain::Error::custom(msg),
            },
        })
    }

    #[test]
    fn u32_above_i32_max() {
        let val: u32 = parse("3000000000").expect("should parse u32 above i32::MAX");
        assert_eq!(val, 3_000_000_000_u32);
    }

    #[test]
    fn u32_max_value() {
        let val: u32 = parse("4294967295").expect("should parse u32::MAX");
        assert_eq!(val, u32::MAX);
    }

    #[test]
    fn i32_positive() {
        let val: i32 = parse("42").expect("should parse positive i32");
        assert_eq!(val, 42_i32);
    }

    #[test]
    fn i32_negative() {
        let val: i32 = parse("-1").expect("should parse negative i32");
        assert_eq!(val, -1_i32);
    }

    #[test]
    fn i32_max_boundary() {
        let val: i32 = parse("2147483647").expect("should parse i32::MAX");
        assert_eq!(val, i32::MAX);
    }

    #[test]
    fn i32_out_of_range() {
        let err = parse::<i32>("3000000000").expect_err("should fail for i32 out of range");
        assert!(
            matches!(err, Error::ParameterOutOfRange { .. }),
            "expected OutOfRange, got: {err:?}"
        );
    }

    #[test]
    fn u32_negative_out_of_range() {
        let err = parse::<u32>("-1").expect_err("should fail for negative u32");
        assert!(
            matches!(err, Error::ParameterOutOfRange { .. }),
            "expected OutOfRange, got: {err:?}"
        );
    }

    #[test]
    fn u8_out_of_range() {
        let err = parse::<u8>("256").expect_err("should fail for u8 out of range");
        assert!(
            matches!(err, Error::ParameterOutOfRange { .. }),
            "expected OutOfRange, got: {err:?}"
        );
    }

    #[test]
    fn bad_format_non_numeric() {
        let err = parse::<u32>("abc").expect_err("should fail for non-numeric input");
        assert!(
            matches!(err, Error::BadParameter { .. }),
            "expected BadParameter, got: {err:?}"
        );
    }

    #[test]
    fn whitespace_trimming() {
        let val: u32 = parse("  42  ").expect("should parse with surrounding whitespace");
        assert_eq!(val, 42_u32);
    }
}
