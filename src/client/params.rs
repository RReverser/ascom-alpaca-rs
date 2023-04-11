use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub(crate) enum ActionParams<T> {
    Get(T),
    Put(T),
}

macro_rules! opaque_params {
    ($($key:ident: $value:expr),* $(,)?) => {{
        #[derive(Debug, serde::Serialize)]
        #[allow(non_snake_case)]
        struct Params<$($key),*> {
            $($key: $key,)*
        }

        Params {
            $($key: $value,)*
        }
    }};
}
pub(crate) use opaque_params;
