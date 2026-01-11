pub(super) use time::format_description::well_known::Iso8601;

use std::fmt::Debug;
use std::marker::PhantomData;
use std::time::SystemTime;
use time::macros::format_description;
use time::{OffsetDateTime, format_description};

pub(super) trait FormatWrapper: Debug {
    type Format: 'static + ?Sized;

    const FORMAT: &'static Self::Format;
}

impl FormatWrapper for Iso8601 {
    type Format = Self;

    const FORMAT: &'static Self = &Self::DEFAULT;
}

#[derive(Debug)]
pub(super) struct Fits;

impl FormatWrapper for Fits {
    type Format = [format_description::BorrowedFormatItem<'static>];

    const FORMAT: &'static Self::Format = format_description!(
        "[year]-[month]-[day]T[hour]:[minute]:[second][optional [.[subsecond digits:3]]]"
    );
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

#[cfg(feature = "server")]
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
