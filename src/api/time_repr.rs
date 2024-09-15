use crate::response::ValueResponse;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use std::time::SystemTime;
use time::formatting::Formattable;
use time::macros::format_description;
use time::parsing::Parsable;
use time::{format_description, OffsetDateTime};

pub(crate) trait FormatWrapper: std::fmt::Debug {
    type Format: 'static + ?Sized + Parsable + Formattable;

    const FORMAT: &'static Self::Format;
}

#[derive(Debug)]
pub(crate) struct Iso8601;

impl FormatWrapper for Iso8601 {
    type Format = format_description::well_known::Iso8601;

    const FORMAT: &'static Self::Format = &Self::Format::DEFAULT;
}

#[derive(Debug)]
pub(crate) struct Fits;

impl FormatWrapper for Fits {
    type Format = [format_description::BorrowedFormatItem<'static>];

    const FORMAT: &'static Self::Format = format_description!(
        "[year]-[month]-[day]T[hour]:[minute]:[second][optional [.[subsecond digits:3]]]"
    );
}

#[derive(Debug)]
pub(crate) struct TimeParam<F>(OffsetDateTime, PhantomData<F>);

impl<F> From<SystemTime> for TimeParam<F> {
    fn from(value: SystemTime) -> Self {
        Self(value.into(), PhantomData)
    }
}

impl<F> From<TimeParam<F>> for SystemTime {
    fn from(wrapper: TimeParam<F>) -> Self {
        wrapper.0.into()
    }
}

impl<F: FormatWrapper> Serialize for TimeParam<F> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0
            .format(&F::FORMAT)
            .map_err(serde::ser::Error::custom)?
            .serialize(serializer)
    }
}

impl<'de, F: FormatWrapper> Deserialize<'de> for TimeParam<F> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor<F>(PhantomData<F>);

        impl<F: FormatWrapper> serde::de::Visitor<'_> for Visitor<F> {
            type Value = TimeParam<F>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a date string")
            }

            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
                match time::PrimitiveDateTime::parse(value, &F::FORMAT) {
                    Ok(value) => Ok(TimeParam(value.assume_utc(), PhantomData)),
                    Err(err) => Err(serde::de::Error::custom(err)),
                }
            }
        }

        deserializer.deserialize_str(Visitor(PhantomData))
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(bound = "F: FormatWrapper")]
#[serde(transparent)]
pub(crate) struct TimeResponse<F>(ValueResponse<TimeParam<F>>);

impl<F> From<SystemTime> for TimeResponse<F> {
    fn from(value: SystemTime) -> Self {
        Self(TimeParam::from(value).into())
    }
}

impl<F> From<TimeResponse<F>> for SystemTime {
    fn from(wrapper: TimeResponse<F>) -> Self {
        wrapper.0.into().into()
    }
}
