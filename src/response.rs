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
    #[allow(clippy::missing_const_for_fn)] // https://github.com/rust-lang/rust-clippy/issues/9271
    pub(crate) fn into_inner(self) -> T {
        self.value
    }
}
