pub(super) use time::format_description::well_known::Iso8601;

use std::fmt::Debug;
use std::marker::PhantomData;
use std::time::{Duration, SystemTime};
use time::OffsetDateTime;

pub(super) trait FormatWrapper: Debug {
    type Format: 'static + ?Sized;

    const FORMAT: &'static Self::Format;
}

impl FormatWrapper for Iso8601 {
    type Format = Self;

    const FORMAT: &'static Self = &Self::DEFAULT;
}

#[derive(Debug)]
pub(super) struct TimeRepr<F>(OffsetDateTime, PhantomData<F>);

impl<F> From<SystemTime> for TimeRepr<F> {
    fn from(value: SystemTime) -> Self {
        Self(value.into(), PhantomData)
    }
}

impl<F> From<TimeRepr<F>> for SystemTime {
    fn from(wrapper: TimeRepr<F>) -> Self {
        wrapper.0.into()
    }
}

impl<F: FormatWrapper> serde::Serialize for TimeRepr<F>
where
    F::Format: time::formatting::Formattable,
{
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0
            .format(F::FORMAT)
            .map_err(serde::ser::Error::custom)?
            .serialize(serializer)
    }
}

impl<'de, F: FormatWrapper> serde::Deserialize<'de> for TimeRepr<F>
where
    F::Format: time::parsing::Parsable,
{
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de;
        use std::fmt::{self, Formatter};

        struct Visitor<F>(PhantomData<F>);

        impl<F: FormatWrapper> de::Visitor<'_> for Visitor<F>
        where
            F::Format: time::parsing::Parsable,
        {
            type Value = TimeRepr<F>;

            fn expecting(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
                formatter.write_str("a date string")
            }

            fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
                match time::PrimitiveDateTime::parse(value, F::FORMAT) {
                    Ok(value) => Ok(TimeRepr(value.assume_utc(), PhantomData)),
                    Err(err) => Err(de::Error::custom(err)),
                }
            }
        }

        deserializer.deserialize_str(Visitor(PhantomData))
    }
}

/// A wrapper that serializes `Duration` as integer milliseconds and deserializes
/// it from a (possibly fractional, for backwards compatibility) millisecond number.
#[derive(derive_more::From, derive_more::Into)]
pub(super) struct DurationInMs(Duration);

#[cfg(feature = "client")]
impl serde::Serialize for DurationInMs {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // Alpaca defines PulseGuide `Duration` as Int32 milliseconds. Emitting a
        // fractional value like `2000.0` makes spec-compliant devices (which bind
        // the field to a 32-bit integer) reject the request with HTTP 400, so we
        // serialize integer milliseconds. `as_millis()` returns `u128`, which
        // serializes as a plain integer string (e.g. `2000`).
        self.0.as_millis().serialize(serializer)
    }
}

#[cfg(feature = "server")]
impl<'de> serde::Deserialize<'de> for DurationInMs {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let ms = f64::deserialize(deserializer)?;
        Ok(Self(
            Duration::try_from_secs_f64(ms / 1000.0).map_err(serde::de::Error::custom)?,
        ))
    }
}

#[cfg(all(test, feature = "client"))]
mod tests {
    use super::DurationInMs;
    use std::time::Duration;

    // Alpaca PulseGuide `Duration` is Int32 milliseconds, so the wire value must be
    // an integer string. A fractional form like `2000.0`/`1.0` is rejected (HTTP 400)
    // by spec-compliant devices that bind the field to a 32-bit integer. serde_json
    // is a faithful proxy here: the integer-vs-float choice lives in the `Serialize`
    // impl, so it shows up identically in any serializer (including the urlencoded
    // body the client actually sends).
    #[test]
    fn serializes_as_integer_milliseconds() {
        // 2 s = 2000 ms, which previously serialized as `2000.0` and was rejected
        // (HTTP 400) by strict Alpaca devices; it must now be the integer `2000`.
        let two_seconds = serde_json::to_string(&DurationInMs::from(Duration::from_secs(2)))
            .expect("duration serializes");
        assert_eq!(two_seconds, "2000");

        // 1 ms previously serialized as `1.0`, which the Omni Simulator rejected.
        let one_milli = serde_json::to_string(&DurationInMs::from(Duration::from_millis(1)))
            .expect("duration serializes");
        assert_eq!(one_milli, "1");
    }
}
