use indexmap::IndexMap;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(transparent)]
pub(crate) struct OpaqueParams(IndexMap<&'static str, String>);

impl From<IndexMap<&'static str, String>> for OpaqueParams {
    fn from(map: IndexMap<&'static str, String>) -> Self {
        Self(map)
    }
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub(crate) enum ActionParams {
    Get(OpaqueParams),
    Put(OpaqueParams),
}

macro_rules! opaque_params {
    ($($key:ident: $value:expr),* $(,)?) => {{
        #[allow(non_snake_case)]
        struct _AssertNoDuplicates {
            $($key: (),)*
        }

        $crate::client::OpaqueParams::from(indexmap::indexmap! {
            $(stringify!($key) => $crate::params::ASCOMParam::to_string($value),)*
        })
    }};
}
pub(crate) use opaque_params;
