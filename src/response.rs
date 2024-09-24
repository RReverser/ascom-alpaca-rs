use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ValueResponse<T> {
    #[serde(rename = "Value")]
    pub(crate) value: T,
}
