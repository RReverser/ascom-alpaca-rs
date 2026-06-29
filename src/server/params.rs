use super::Error;
use super::case_insensitive_str::CaseInsensitiveStr;
use axum::Form;
use axum::extract::{FromRequest, Request};
use axum::response::{IntoResponse, Response};
use http::{Method, StatusCode};
use indexmap::IndexMap;
use serde::Deserialize;
use serde::de::{DeserializeOwned, Deserializer, Expected, Unexpected, Visitor};
use std::fmt::{self, Debug};
use std::hash::Hash;

/// Error type for Alpaca parameter parsing that distinguishes malformed input
/// (HTTP 400) from values that parse but are semantically rejected (`INVALID_VALUE`).
#[derive(Debug, thiserror::Error)]
pub(crate) enum AlpacaParseError {
    /// Invalid format (not a valid integer/bool/etc) -> HTTP 400
    #[error("{0}")]
    BadFormat(String),
    /// A value that parsed but the target rejected: an out-of-range integer or
    /// an unknown `serde_repr` variant -> ASCOM `INVALID_VALUE`.
    #[error("{0}")]
    InvalidValue(String),
}

impl serde::de::Error for AlpacaParseError {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        // serde calls this when a value parsed but the target rejected it: an
        // out-of-range integer, an unknown `serde_repr` variant, or a
        // `Deserialize` impl calling `Error::custom`. All map to `INVALID_VALUE`.
        Self::InvalidValue(msg.to_string())
    }

    fn invalid_type(unexp: Unexpected<'_>, exp: &dyn Expected) -> Self {
        // The value is the wrong *kind* for the target (non-boolean string for a
        // `bool`, non-integer for an integer, an unsupported shape): a malformed
        // request -> HTTP 400, unlike `invalid_value`/`custom` (-> `INVALID_VALUE`).
        Self::BadFormat(format!("invalid type: {unexp}, expected {exp}"))
    }
}

/// Custom serde Deserializer for Alpaca parameters.
///
/// Integers are parsed once as `i64` and handed to the target type's visitor,
/// which narrows them in a checked way. This covers both i32 device parameters
/// and uint32 transaction/identity parameters (`ClientID`, `ClientTransactionID`)
/// without per-type dispatch.
struct AlpacaDeserializer {
    value: String,
}

const ALPACA_PARAM: &dyn Expected = &"an ASCOM Alpaca-compliant parameter";

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
            Err(serde::de::Error::invalid_type(
                Unexpected::Str(&self.value),
                &"a boolean",
            ))
        }
    }

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
        let value: i64 = self.value.parse().map_err(|_parse_err| {
            serde::de::Error::invalid_type(Unexpected::Str(&self.value), &"an integer")
        })?;
        visitor.visit_i64(value)
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
            serde::de::Error::invalid_type(Unexpected::Str(&self.value), &"a float")
        })?;
        visitor.visit_f32(value)
    }

    fn deserialize_f64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let value: f64 = self.value.parse().map_err(|_parse_err| {
            serde::de::Error::invalid_type(Unexpected::Str(&self.value), &"a float")
        })?;
        visitor.visit_f64(value)
    }

    fn deserialize_char<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_str(visitor)
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
        // Parse via i64 so a negative discriminant like -1 is rejected as
        // INVALID_VALUE (see `plain_enum_negative_index`).
        let discriminant: i64 = self.value.parse().map_err(|_parse_err| {
            serde::de::Error::invalid_type(Unexpected::Str(&self.value), &"an integer")
        })?;
        let index = u32::try_from(discriminant).map_err(|_overflow| {
            serde::de::Error::invalid_value(
                Unexpected::Signed(discriminant),
                &"a valid enum value (u32)",
            )
        })?;
        visitor.visit_enum(serde::de::value::U32Deserializer::new(index))
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_string(self.value)
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_string(self.value)
    }

    fn deserialize_option<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Self::Error> {
        Err(serde::de::Error::invalid_type(
            Unexpected::Option,
            ALPACA_PARAM,
        ))
    }

    fn deserialize_unit<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Self::Error> {
        Err(serde::de::Error::invalid_type(
            Unexpected::Unit,
            ALPACA_PARAM,
        ))
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _visitor: V,
    ) -> Result<V::Value, Self::Error> {
        Err(serde::de::Error::invalid_type(
            Unexpected::Unit,
            ALPACA_PARAM,
        ))
    }

    fn deserialize_seq<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Self::Error> {
        Err(serde::de::Error::invalid_type(
            Unexpected::Seq,
            ALPACA_PARAM,
        ))
    }

    fn deserialize_tuple<V: Visitor<'de>>(
        self,
        _len: usize,
        _visitor: V,
    ) -> Result<V::Value, Self::Error> {
        Err(serde::de::Error::invalid_type(
            Unexpected::Seq,
            ALPACA_PARAM,
        ))
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _len: usize,
        _visitor: V,
    ) -> Result<V::Value, Self::Error> {
        Err(serde::de::Error::invalid_type(
            Unexpected::Seq,
            ALPACA_PARAM,
        ))
    }

    fn deserialize_map<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Self::Error> {
        Err(serde::de::Error::invalid_type(
            Unexpected::Map,
            ALPACA_PARAM,
        ))
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Self::Error> {
        Err(serde::de::Error::invalid_type(
            Unexpected::Map,
            ALPACA_PARAM,
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

    #[test]
    fn bad_format_non_boolean() {
        // A string that isn't a boolean is the wrong *kind* of input -> HTTP 400,
        // routed through serde's `invalid_type` like a non-numeric integer.
        let err = parse::<bool>("maybe").expect_err("should fail for non-boolean input");
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

    #[test]
    fn bool_is_case_insensitive() {
        assert!(parse::<bool>("True").expect("mixed-case true"));
        assert!(!parse::<bool>("FALSE").expect("upper-case false"));
    }

    #[test]
    fn bad_format_non_float() {
        let err = parse::<f64>("abc").expect_err("should fail for non-float input");
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

    #[test]
    fn unsupported_composite_type_is_bad_format() {
        // Composite Rust types have no scalar Alpaca wire representation, so a
        // parameter typed as one is rejected as malformed -> HTTP 400.
        let err = parse::<Vec<u32>>("5").expect_err("sequences aren't valid Alpaca parameters");
        let Error::BadParameter {
            err: AlpacaParseError::BadFormat(msg),
            ..
        } = &err
        else {
            panic!("expected BadFormat, got: {err:?}");
        };
        assert_eq!(
            msg.as_str(),
            "invalid type: sequence, expected an ASCOM Alpaca-compliant parameter"
        );
    }

    // serde_repr emits `Error::custom("invalid value: N, expected one of: ...")`
    // when an integer doesn't match any variant. Without the visitor-error
    // promotion, that would surface as `BadFormat` -> HTTP 400; ConformU's
    // `TrackingRate Write` test (which sends `5` and `-1` for DriveRate) flagged
    // exactly this path. It must produce `InvalidValue` -> ASCOM `INVALID_VALUE`.
    #[derive(serde_repr::Deserialize_repr, Debug)]
    #[repr(i32)]
    enum DriveRateTestEnum {
        Sidereal = 0,
        Lunar = 1,
        Solar = 2,
        King = 3,
    }

    #[test]
    fn enum_repr_unknown_positive_variant() {
        let err =
            parse::<DriveRateTestEnum>("5").expect_err("should fail for out-of-variant value");
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
        let err = parse::<DriveRateTestEnum>("-1")
            .expect_err("should fail for negative out-of-variant value");
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
        let val: DriveRateTestEnum = parse("1").expect("should parse valid variant");
        assert!(matches!(val, DriveRateTestEnum::Lunar));
    }

    #[derive(serde::Deserialize, Debug, PartialEq)]
    enum PlainEnum {
        Alpha,
        Beta,
    }

    #[test]
    fn plain_enum_index_selects_variant() {
        let val: PlainEnum = parse("1").expect("integer should select the variant by index");
        assert_eq!(val, PlainEnum::Beta);
    }

    #[test]
    fn plain_enum_index_zero() {
        let val: PlainEnum = parse("0").expect("integer should select the variant by index");
        assert_eq!(val, PlainEnum::Alpha);
    }

    #[test]
    fn plain_enum_variant_name_rejected() {
        // Enums decode as integers, so a string variant name is a format error
        // (HTTP 400) — the same way a `serde_repr` enum rejects a non-integer.
        let err = parse::<PlainEnum>("Beta").expect_err("variant name should not be accepted");
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

    #[test]
    fn plain_enum_index_out_of_range() {
        let err = parse::<PlainEnum>("5").expect_err("index past the last variant should fail");
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
    fn plain_enum_negative_index() {
        let err = parse::<PlainEnum>("-1").expect_err("negative index should fail");
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
}
