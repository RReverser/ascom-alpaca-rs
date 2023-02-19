use anyhow::Context;

use indexmap::IndexMap;
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt::Debug;

pub(crate) trait ASCOMParam: Sized {
    fn from_string(s: String) -> anyhow::Result<Self>;
    fn to_string(self) -> String;
}

impl ASCOMParam for String {
    fn from_string(s: String) -> anyhow::Result<Self> {
        Ok(s)
    }

    fn to_string(self) -> String {
        self
    }
}

impl ASCOMParam for bool {
    fn from_string(s: String) -> anyhow::Result<Self> {
        Ok(match s.as_str() {
            "True" => true,
            "False" => false,
            _ => anyhow::bail!(r#"Invalid bool value {s:?}, expected "True" or "False""#),
        })
    }

    fn to_string(self) -> String {
        (if self { "True" } else { "False" }).to_owned()
    }
}

macro_rules! simple_ascom_param {
    ($($ty:ty),*) => {
        $(
            impl ASCOMParam for $ty {
                fn from_string(s: String) -> anyhow::Result<Self> {
                    Ok(s.parse()?)
                }

                fn to_string(self) -> String {
                    ToString::to_string(&self)
                }
            }
        )*
    };
}

simple_ascom_param!(i32, u32, f64);

macro_rules! ascom_enum {
    ($name:ty) => {
        impl $crate::params::ASCOMParam for $name {
            fn from_string(s: String) -> anyhow::Result<Self> {
                Ok(<Self as num_enum::TryFromPrimitive>::try_from_primitive(
                    $crate::params::ASCOMParam::from_string(s)?,
                )?)
            }

            fn to_string(self) -> String {
                let primitive: <Self as num_enum::TryFromPrimitive>::Primitive = self.into();
                $crate::params::ASCOMParam::to_string(primitive)
            }
        }
    };
}
pub(crate) use ascom_enum;

#[derive(Serialize)]
#[serde(transparent)]
#[repr(transparent)]
pub(crate) struct CaseInsensitiveStr(str);

impl AsRef<CaseInsensitiveStr> for str {
    fn as_ref(&self) -> &CaseInsensitiveStr {
        #[allow(clippy::as_conversions)]
        unsafe {
            &*(self as *const _ as *const _)
        }
    }
}

impl From<Box<str>> for Box<CaseInsensitiveStr> {
    fn from(str: Box<str>) -> Self {
        let as_ptr = Box::into_raw(str);
        #[allow(clippy::as_conversions)]
        unsafe {
            Self::from_raw(as_ptr as *mut _)
        }
    }
}

impl<'de> Deserialize<'de> for Box<CaseInsensitiveStr> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        <Box<str>>::deserialize(deserializer).map(Into::into)
    }
}

impl Debug for CaseInsensitiveStr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl PartialEq for CaseInsensitiveStr {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq_ignore_ascii_case(&other.0)
    }
}

impl Eq for CaseInsensitiveStr {}

impl std::hash::Hash for CaseInsensitiveStr {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        for b in self.0.as_bytes() {
            state.write_u8(b.to_ascii_lowercase());
        }
        state.write_u8(0xff);
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub(crate) struct OpaqueParams(pub(crate) IndexMap<Box<CaseInsensitiveStr>, String>);

impl OpaqueParams {
    pub(crate) fn maybe_extract<T: ASCOMParam>(&mut self, name: &str) -> anyhow::Result<Option<T>> {
        self.0
            .remove::<CaseInsensitiveStr>(name.as_ref())
            .map(|value| {
                T::from_string(value).with_context(|| format!("Invalid value for parameter {name}"))
            })
            .transpose()
    }

    pub(crate) fn extract<T: ASCOMParam>(&mut self, name: &str) -> anyhow::Result<T> {
        self.maybe_extract(name)?
            .ok_or_else(|| anyhow::anyhow!("Missing parameter {name}"))
    }

    pub(crate) fn finish_extraction(self) {
        if !self.0.is_empty() {
            tracing::warn!("Unexpected parameters: {:?}", self.0.keys());
        }
    }

    pub(crate) fn insert<T: ASCOMParam>(&mut self, name: &str, value: T) {
        let prev_value = self
            .0
            .insert(Box::<str>::from(name).into(), value.to_string());
        debug_assert!(prev_value.is_none());
    }
}
