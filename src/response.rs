use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ValueResponse<T> {
    #[serde(rename = "Value")]
    value: T,
}

impl<T> From<T> for ValueResponse<T> {
    fn from(value: T) -> Self {
        Self { value }
    }
}

impl<T> ValueResponse<T> {
    pub(crate) fn into(self) -> T {
        self.value
    }
}
