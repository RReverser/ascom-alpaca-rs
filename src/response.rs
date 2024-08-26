use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ValueResponse<T> {
    #[serde(rename = "Value")]
    value: T,
}

#[cfg(feature = "server")]
impl<T> From<T> for ValueResponse<T> {
    fn from(value: T) -> Self {
        Self { value }
    }
}

#[cfg(feature = "client")]
impl<T> ValueResponse<T> {
    pub(crate) fn into(self) -> T {
        self.value
    }
}
