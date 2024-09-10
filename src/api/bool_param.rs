use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;

#[derive(Serialize)]
#[serde(transparent)]
pub(crate) struct BoolParam(bool);

impl fmt::Debug for BoolParam {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<bool> for BoolParam {
    fn from(b: bool) -> Self {
        Self(b)
    }
}

impl From<BoolParam> for bool {
    fn from(b: BoolParam) -> Self {
        b.0
    }
}

impl<'de> Deserialize<'de> for BoolParam {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl serde::de::Visitor<'_> for Visitor {
            type Value = bool;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("'true' or 'false' in any casing")
            }

            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Self::Value, E> {
                if v.eq_ignore_ascii_case("true") {
                    Ok(true)
                } else if v.eq_ignore_ascii_case("false") {
                    Ok(false)
                } else {
                    Err(E::invalid_value(serde::de::Unexpected::Str(v), &self))
                }
            }
        }

        deserializer.deserialize_str(Visitor).map(Self)
    }
}
