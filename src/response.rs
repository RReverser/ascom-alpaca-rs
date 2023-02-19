use crate::{ASCOMError, ASCOMErrorCode};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct OpaqueResponse(pub(crate) serde_json::Map<String, serde_json::Value>);

impl OpaqueResponse {
    pub(crate) fn new<T: Debug + Serialize>(value: T) -> Self {
        let json = serde_json::to_value(&value).unwrap_or_else(|err| {
            // This should never happen, but if it does, log and return the error.
            // This simplifies error handling for this rare case without having to panic!.
            tracing::error!(?value, %err, "Serialization failure");
            serde_json::to_value(ASCOMError {
                code: ASCOMErrorCode::UNSPECIFIED,
                message: format!("Failed to serialize {value:#?}: {err}").into(),
            })
            .expect("ASCOMError can never fail to serialize")
        });

        Self(match json {
            serde_json::Value::Object(map) => map,
            serde_json::Value::Null => serde_json::Map::new(),
            value => {
                // Wrap into IntResponse / BoolResponse / ..., aka {"value": ...}
                std::iter::once(("Value".to_owned(), value)).collect()
            }
        })
    }

    pub(crate) fn try_as<T: DeserializeOwned>(mut self) -> serde_json::Result<T> {
        serde_json::from_value(if self.0.contains_key("Value") {
            #[allow(clippy::unwrap_used)]
            self.0.remove("Value").unwrap()
        } else {
            serde_json::Value::Object(self.0)
        })
    }
}
