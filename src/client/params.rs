use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub(crate) enum ActionParams<T> {
    Get(T),
    Put(T),
}
