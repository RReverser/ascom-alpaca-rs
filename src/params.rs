use anyhow::Context;
use axum::body::HttpBody;
use axum::extract::FromRequest;
use axum::http::{Method, Request, StatusCode};
use axum::response::IntoResponse;
use axum::{BoxError, Form};
use indexmap::IndexMap;
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt::Debug;
use std::hash::Hash;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

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
    fn from(s: Box<str>) -> Self {
        let as_ptr = Box::into_raw(s);
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

impl Hash for CaseInsensitiveStr {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        for b in self.0.as_bytes() {
            state.write_u8(b.to_ascii_lowercase());
        }
        state.write_u8(0xff);
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
#[serde(bound(
    serialize = "ParamStr: Serialize + Hash + Eq",
    deserialize = "Box<ParamStr>: serde::de::DeserializeOwned + Hash + Eq"
))]
pub(crate) struct OpaqueParams<ParamStr: ?Sized + Debug>(
    pub(crate) IndexMap<Box<ParamStr>, String>,
);

impl<ParamStr: ?Sized + Debug> Default for OpaqueParams<ParamStr> {
    fn default() -> Self {
        Self(IndexMap::new())
    }
}

impl<ParamStr: ?Sized + Debug + Hash + Eq> OpaqueParams<ParamStr>
where
    str: AsRef<ParamStr>,
{
    pub(crate) fn maybe_extract<T: ASCOMParam>(&mut self, name: &str) -> anyhow::Result<Option<T>> {
        self.0
            .remove(name.as_ref())
            .map(|value| {
                T::from_string(value).with_context(|| format!("Invalid value for parameter {name}"))
            })
            .transpose()
    }

    pub(crate) fn extract<T: ASCOMParam>(&mut self, name: &str) -> anyhow::Result<T> {
        self.maybe_extract(name)?
            .ok_or_else(|| anyhow::anyhow!("Missing parameter {name}"))
    }

    pub(crate) fn insert<T: ASCOMParam>(&mut self, name: &str, value: T)
    where
        Box<ParamStr>: From<Box<str>>,
    {
        let prev_value = self
            .0
            .insert(Box::<str>::from(name).into(), value.to_string());
        debug_assert!(prev_value.is_none());
    }
}

impl<ParamStr: ?Sized + Debug> Drop for OpaqueParams<ParamStr> {
    fn drop(&mut self) {
        if !self.0.is_empty() {
            tracing::warn!("Unused parameters: {:?}", self.0.keys());
        }
    }
}

#[derive(Debug)]
pub(crate) enum RawActionParams {
    Get(OpaqueParams<CaseInsensitiveStr>),
    Put(OpaqueParams<str>),
}

#[derive(Debug)]
pub(crate) enum DeviceActionParams<'device, Device: ?Sized> {
    Get {
        device: RwLockReadGuard<'device, Device>,
        params: OpaqueParams<CaseInsensitiveStr>,
    },
    Put {
        device: RwLockWriteGuard<'device, Device>,
        params: OpaqueParams<str>,
    },
}

impl RawActionParams {
    pub(crate) fn maybe_extract<T: ASCOMParam>(&mut self, name: &str) -> anyhow::Result<Option<T>> {
        match self {
            Self::Get(params) => params.maybe_extract(name),
            Self::Put(params) => params.maybe_extract(name),
        }
    }
}

impl<'device, Device: ?Sized + crate::api::Device> DeviceActionParams<'device, Device> {
    pub(crate) async fn new(
        device: &'device RwLock<Device>,
        raw_params: RawActionParams,
    ) -> DeviceActionParams<'device, Device> {
        match raw_params {
            RawActionParams::Get(params) => Self::Get {
                device: device.read().await,
                params,
            },
            RawActionParams::Put(params) => Self::Put {
                device: device.write().await,
                params,
            },
        }
    }
}

#[async_trait::async_trait]
impl<S, B> FromRequest<S, B> for RawActionParams
where
    B: HttpBody + Send + Sync + 'static,
    B::Data: Send,
    B::Error: Into<BoxError>,
    S: Send + Sync,
{
    type Rejection = axum::response::Response;

    async fn from_request(req: Request<B>, state: &S) -> Result<Self, Self::Rejection> {
        match *req.method() {
            Method::GET => Ok(Self::Get(
                Form::from_request(req, state)
                    .await
                    .map_err(IntoResponse::into_response)?
                    .0,
            )),
            Method::PUT => Ok(Self::Put(
                Form::from_request(req, state)
                    .await
                    .map_err(IntoResponse::into_response)?
                    .0,
            )),
            _ => Err((StatusCode::METHOD_NOT_ALLOWED, "Method not allowed").into_response()),
        }
    }
}
