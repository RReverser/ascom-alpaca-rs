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
            visitor.visit_string(self.value)
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
        match self.value.parse::<i64>() {
            Ok(value) => visitor.visit_i64(value),
            Err(_) => visitor.visit_string(self.value),
        }
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
        match self.value.parse::<f32>() {
            Ok(value) => visitor.visit_f32(value),
            Err(_) => visitor.visit_string(self.value),
        }
    }

    fn deserialize_f64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.value.parse::<f64>() {
            Ok(value) => visitor.visit_f64(value),
            Err(_) => visitor.visit_string(self.value),
        }
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
        // Alpaca enums are integers. Parse as i64 so a negative value is rejected
        // as INVALID_VALUE, not a malformed 400; non-negative indices defer to the
        // derive's visitor, which emits invalid_value for out-of-range variants.
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

    // Every remaining shape has no scalar Alpaca wire representation. Forward
    // them to `deserialize_any`, which offers the raw string to the target
    // visitor; that visitor rejects it as `invalid_type` (-> BadFormat -> 400).
    serde::forward_to_deserialize_any! {
        char str string bytes byte_buf identifier ignored_any
        option unit unit_struct seq tuple tuple_struct map struct
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
        // A string that isn't a boolean is the wrong *kind* of input -> HTTP 400.
        // Handing the raw value to the `bool` visitor yields serde's own
        // `invalid_type` error, identical to a hand-built one.
        let err = parse::<bool>("maybe").expect_err("should fail for non-boolean input");
        let Error::BadParameter {
            err: AlpacaParseError::BadFormat(msg),
            ..
        } = &err
        else {
            panic!("expected BadFormat, got: {err:?}");
        };
        assert_eq!(
            msg.as_str(),
            "invalid type: string \"maybe\", expected a boolean"
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
            "invalid type: string \"5\", expected a sequence"
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
    fn plain_enum_out_of_range_maps_to_invalid_value_via_both_arms() {
        // Both out-of-range indices become INVALID_VALUE (1025), but through two
        // different arms — which is exactly why `deserialize_enum` parses i64
        // first. A non-negative index is handed to `U32Deserializer`, so the
        // derive's own `visit_u64` rejects it; a negative value can't be a u32, so
        // our `try_from` arm rejects it before it could ever reach the derive.
        let positive = parse::<PlainEnum>("5").expect_err("index past the last variant");
        let Error::BadParameter {
            err: AlpacaParseError::InvalidValue(msg),
            ..
        } = &positive
        else {
            panic!("expected InvalidValue for 5, got: {positive:?}");
        };
        assert!(
            msg.contains("variant index"),
            "5 should be rejected by the derive's visit_u64 arm, got: {msg}"
        );

        let negative = parse::<PlainEnum>("-1").expect_err("negative index");
        let Error::BadParameter {
            err: AlpacaParseError::InvalidValue(msg),
            ..
        } = &negative
        else {
            panic!("expected InvalidValue for -1, got: {negative:?}");
        };
        assert!(
            msg.contains("a valid enum value (u32)"),
            "-1 should be rejected by our try_from arm, got: {msg}"
        );

        // A non-integer is a different class entirely: BadFormat -> 400. A direct
        // u32 parse would wrongly collapse `-1` into this same bucket.
        let non_integer = parse::<PlainEnum>("Beta").expect_err("variant name is not an integer");
        assert!(
            matches!(
                non_integer,
                Error::BadParameter {
                    err: AlpacaParseError::BadFormat(_),
                    ..
                }
            ),
            "expected BadFormat for Beta, got: {non_integer:?}"
        );
    }
}
