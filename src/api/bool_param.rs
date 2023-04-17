use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct BoolParam(#[serde(deserialize_with = "BoolParam::deserialize")] bool);

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

impl BoolParam {
    fn deserialize<'de, D>(deserializer: D) -> Result<bool, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Visitor;

        impl serde::de::Visitor<'_> for Visitor {
            type Value = bool;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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

        deserializer.deserialize_str(Visitor)
    }
}
