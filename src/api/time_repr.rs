use crate::response::ValueResponse;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;
use time::format_description::well_known::iso8601::EncodedConfig;
use time::format_description::well_known::Iso8601;

pub(crate) struct TimeParam<const CONFIG: EncodedConfig>(SystemTime);

// `time` crate doesn't expose config from `Iso8601`, so we have to extract one manually via inference
pub(crate) const fn config_from<const CONFIG: EncodedConfig>(_: Iso8601<CONFIG>) -> EncodedConfig {
    CONFIG
}

impl<const CONFIG: EncodedConfig> std::fmt::Debug for TimeParam<CONFIG> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<const CONFIG: EncodedConfig> From<SystemTime> for TimeParam<CONFIG> {
    fn from(value: SystemTime) -> Self {
        Self(value)
    }
}

impl<const CONFIG: EncodedConfig> From<TimeParam<CONFIG>> for SystemTime {
    fn from(wrapper: TimeParam<CONFIG>) -> Self {
        wrapper.0
    }
}

impl<const CONFIG: EncodedConfig> Serialize for TimeParam<CONFIG> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        time::OffsetDateTime::from(self.0)
            .format(&Iso8601::<CONFIG>)
            .map_err(serde::ser::Error::custom)?
            .serialize(serializer)
    }
}

impl<'de, const CONFIG: EncodedConfig> Deserialize<'de> for TimeParam<CONFIG> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor<const CONFIG: EncodedConfig>;

        impl<const CONFIG: EncodedConfig> serde::de::Visitor<'_> for Visitor<CONFIG> {
            type Value = TimeParam<{ CONFIG }>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a date string")
            }

            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
                match time::OffsetDateTime::parse(value, &Iso8601::<CONFIG>) {
                    Ok(value) => Ok(TimeParam(value.into())),
                    Err(err) => Err(serde::de::Error::custom(err)),
                }
            }
        }

        deserializer.deserialize_str(Visitor)
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct TimeResponse<const CONFIG: EncodedConfig>(ValueResponse<TimeParam<CONFIG>>);

impl<const CONFIG: EncodedConfig> From<SystemTime> for TimeResponse<CONFIG> {
    fn from(value: SystemTime) -> Self {
        Self(TimeParam::from(value).into())
    }
}

impl<const CONFIG: EncodedConfig> From<TimeResponse<CONFIG>> for SystemTime {
    fn from(wrapper: TimeResponse<CONFIG>) -> Self {
        wrapper.0.into().into()
    }
}
