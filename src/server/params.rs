use super::Error;
use super::case_insensitive_str::CaseInsensitiveStr;
use axum::Form;
use axum::extract::{FromRequest, Request};
use axum::response::{IntoResponse, Response};
use http::{Method, StatusCode};
use indexmap::IndexMap;
use serde::Deserialize;
use serde::de::{DeserializeOwned, Deserializer, Visitor};
use std::fmt::{self, Debug};
use std::hash::Hash;

/// Error type for Alpaca parameter parsing that distinguishes malformed input
/// (HTTP 400) from values that parse but are semantically rejected (`INVALID_VALUE`).
#[derive(Debug, thiserror::Error)]
pub(crate) enum AlpacaParseError {
    /// Invalid format (not a valid integer/bool/etc) -> HTTP 400
    #[error("{0}")]
    BadFormat(String),
    /// Primitive parsed successfully but the target type rejected the value:
    /// an integer outside the target's range, or one that doesn't match any
    /// variant of a `serde_repr` enum -> ASCOM `INVALID_VALUE`.
    #[error("{0}")]
    InvalidValue(String),
}

impl serde::de::Error for AlpacaParseError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        // Reached when serde semantically rejects a value whose wire bytes
        // parsed fine: an integer outside the target type's range (serde's
        // checked `visit_i64` narrowing), a `serde_repr` enum discriminant
        // matching no variant, or a user `Deserialize` impl calling
        // `Error::custom`. All of these are ASCOM `INVALID_VALUE`. Our own
        // format errors are produced as `BadFormat` directly and never reach here.
        Self::InvalidValue(msg.to_string())
    }
}

/// Custom serde Deserializer for Alpaca parameters.
///
/// Integers are parsed once as `i64` and handed to the target type's visitor,
/// which narrows them in a checked way (out-of-range -> `INVALID_VALUE`). This
/// covers both i32 device parameters and uint32 transaction/identity parameters
/// (`ClientID`, `ClientTransactionID`) without per-type dispatch.
struct AlpacaDeserializer {
    value: String,
}

impl AlpacaDeserializer {
    /// Parse the value as `i64`. Every integer `deserialize_*` method forwards
    /// here and hands the result to `visit_i64`; serde's built-in integer
    /// `Deserialize` impls then narrow to the target type in a checked way,
    /// mapping out-of-range values to `INVALID_VALUE` via `Error::custom`.
    fn parse_i64(&self) -> Result<i64, AlpacaParseError> {
        self.value.parse().map_err(|_parse_err| {
            AlpacaParseError::BadFormat(format!("invalid integer: {}", self.value))
        })
    }
}

impl<'de> Deserializer<'de> for AlpacaDeserializer {
    type Error = AlpacaParseError;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_string(self.value)
    }

    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        // Alpaca booleans may use any casing; compare in place rather than
        // allocating a lowercased copy of the value we already own.
        if self.value.eq_ignore_ascii_case("true") {
            visitor.visit_bool(true)
        } else if self.value.eq_ignore_ascii_case("false") {
            visitor.visit_bool(false)
        } else {
            Err(AlpacaParseError::BadFormat(format!(
                "invalid boolean: {}",
                self.value
            )))
        }
    }

    // All integer widths funnel through `deserialize_i64`: parse once as i64
    // and let serde's target-type visitor narrow it in a checked way (an
    // out-of-range value becomes `INVALID_VALUE`, never a silent truncation).
    fn deserialize_i8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_i64(visitor)
    }

    fn deserialize_i16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_i64(visitor)
    }

    fn deserialize_i32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_i64(visitor)
    }

    fn deserialize_i64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_i64(self.parse_i64()?)
    }

    fn deserialize_u8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_i64(visitor)
    }

    fn deserialize_u16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_i64(visitor)
    }

    fn deserialize_u32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_i64(visitor)
    }

    fn deserialize_u64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_i64(visitor)
    }

    fn deserialize_f32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let value: f32 = self.value.parse().map_err(|_parse_err| {
            AlpacaParseError::BadFormat(format!("invalid float: {}", self.value))
        })?;
        visitor.visit_f32(value)
    }

    fn deserialize_f64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let value: f64 = self.value.parse().map_err(|_parse_err| {
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
            "Option types are not supported as Alpaca parameters".to_owned(),
        ))
    }

    fn deserialize_unit<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Self::Error> {
        Err(AlpacaParseError::BadFormat(
            "unit type is not supported as Alpaca parameter".to_owned(),
        ))
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _visitor: V,
    ) -> Result<V::Value, Self::Error> {
        Err(AlpacaParseError::BadFormat(
            "unit struct is not supported as Alpaca parameter".to_owned(),
        ))
    }

    fn deserialize_seq<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Self::Error> {
        Err(AlpacaParseError::BadFormat(
            "sequences are not supported as Alpaca parameters".to_owned(),
        ))
    }

    fn deserialize_tuple<V: Visitor<'de>>(
        self,
        _len: usize,
        _visitor: V,
    ) -> Result<V::Value, Self::Error> {
        Err(AlpacaParseError::BadFormat(
            "tuples are not supported as Alpaca parameters".to_owned(),
        ))
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _len: usize,
        _visitor: V,
    ) -> Result<V::Value, Self::Error> {
        Err(AlpacaParseError::BadFormat(
            "tuple structs are not supported as Alpaca parameters".to_owned(),
        ))
    }

    fn deserialize_map<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Self::Error> {
        Err(AlpacaParseError::BadFormat(
            "maps are not supported as Alpaca parameters".to_owned(),
        ))
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Self::Error> {
        Err(AlpacaParseError::BadFormat(
            "structs are not supported as Alpaca parameters".to_owned(),
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
                T::deserialize(AlpacaDeserializer { value })
                    .map_err(|err| Error::BadParameter { name, err })
            })
            .transpose()
    }

    pub(crate) fn extract<T: DeserializeOwned>(
        &mut self,
        name: &'static str,
    ) -> super::Result<T> {
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
        T::deserialize(deser).map_err(|err| Error::BadParameter { name: "test", err })
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
            matches!(
                err,
                Error::BadParameter {
                    err: AlpacaParseError::InvalidValue(_),
                    ..
                }
            ),
            "expected InvalidValue, got: {err:?}"
        );
    }

    #[test]
    fn u32_negative_out_of_range() {
        let err = parse::<u32>("-1").expect_err("should fail for negative u32");
        assert!(
            matches!(
                err,
                Error::BadParameter {
                    err: AlpacaParseError::InvalidValue(_),
                    ..
                }
            ),
            "expected InvalidValue, got: {err:?}"
        );
    }

    #[test]
    fn u8_out_of_range() {
        let err = parse::<u8>("256").expect_err("should fail for u8 out of range");
        assert!(
            matches!(
                err,
                Error::BadParameter {
                    err: AlpacaParseError::InvalidValue(_),
                    ..
                }
            ),
            "expected InvalidValue, got: {err:?}"
        );
    }

    #[test]
    fn bad_format_non_numeric() {
        let err = parse::<u32>("abc").expect_err("should fail for non-numeric input");
        assert!(
            matches!(
                err,
                Error::BadParameter {
                    err: AlpacaParseError::BadFormat(_),
                    ..
                }
            ),
            "expected BadFormat, got: {err:?}"
        );
    }

    // serde_repr emits `Error::custom("invalid value: N, expected one of: ...")`
    // when an integer doesn't match any variant. Without the visitor-error
    // promotion, that would surface as `BadFormat` -> HTTP 400; ConformU's
    // `TrackingRate Write` test (which sends `5` and `-1` for DriveRate) flagged
    // exactly this path. It must produce `InvalidValue` -> ASCOM `INVALID_VALUE`.
    #[derive(serde_repr::Deserialize_repr, Debug)]
    #[repr(i32)]
    enum ReprEnum {
        Zero = 0,
        One = 1,
        Two = 2,
    }

    #[test]
    fn enum_repr_unknown_positive_variant() {
        let err = parse::<ReprEnum>("5").expect_err("should fail for out-of-variant value");
        assert!(
            matches!(
                err,
                Error::BadParameter {
                    err: AlpacaParseError::InvalidValue(_),
                    ..
                }
            ),
            "expected InvalidValue, got: {err:?}"
        );
    }

    #[test]
    fn enum_repr_unknown_negative_variant() {
        let err = parse::<ReprEnum>("-1").expect_err("should fail for negative out-of-variant value");
        assert!(
            matches!(
                err,
                Error::BadParameter {
                    err: AlpacaParseError::InvalidValue(_),
                    ..
                }
            ),
            "expected InvalidValue, got: {err:?}"
        );
    }

    #[test]
    fn enum_repr_known_variant_round_trips() {
        let val: ReprEnum = parse("1").expect("should parse valid variant");
        assert!(matches!(val, ReprEnum::One));
    }
}
