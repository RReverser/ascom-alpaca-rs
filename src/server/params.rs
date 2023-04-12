use super::case_insensitive_str::CaseInsensitiveStr;
use anyhow::Context;
use axum::body::HttpBody;
use axum::extract::FromRequest;
use axum::http::{Method, Request, StatusCode};
use axum::response::IntoResponse;
use axum::{BoxError, Form};
use indexmap::IndexMap;
use serde::Deserialize;
use std::fmt::Debug;
use std::hash::Hash;

#[derive(Debug, Deserialize)]
#[serde(transparent)]
#[serde(bound(deserialize = "Box<ParamStr>: serde::de::DeserializeOwned + Hash + Eq"))]
pub(crate) struct OpaqueParams<ParamStr: ?Sized + Debug>(IndexMap<Box<ParamStr>, String>);

impl<ParamStr: ?Sized + Debug> Drop for OpaqueParams<ParamStr> {
    fn drop(&mut self) {
        if !self.0.is_empty() {
            tracing::warn!("Unused parameters: {:?}", self.0);
        }
    }
}

#[derive(Debug)]
pub(crate) enum ActionParams {
    Get(OpaqueParams<CaseInsensitiveStr>),
    Put(OpaqueParams<str>),
}

impl<ParamStr: ?Sized + Hash + Eq + Debug> OpaqueParams<ParamStr>
where
    str: AsRef<ParamStr>,
{
    pub(crate) fn maybe_extract<T: ASCOMParam>(&mut self, name: &str) -> anyhow::Result<Option<T>> {
        self.0
            .remove(name.as_ref())
            .map(|value| {
                T::from_string(value)
                    .with_context(|| format!("Invalid value for parameter {name:?}"))
            })
            .transpose()
    }

    pub(crate) fn extract<T: ASCOMParam>(&mut self, name: &str) -> anyhow::Result<T> {
        self.maybe_extract(name)?
            .ok_or_else(|| anyhow::anyhow!("Missing parameter {name:?}"))
    }
}

#[async_trait::async_trait]
impl<S, B> FromRequest<S, B> for ActionParams
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

pub(crate) trait ASCOMParam: Sized {
    fn from_string(s: String) -> anyhow::Result<Self>;
}

impl ASCOMParam for String {
    fn from_string(s: String) -> anyhow::Result<Self> {
        Ok(s)
    }
}

impl ASCOMParam for bool {
    fn from_string(s: String) -> anyhow::Result<Self> {
        Ok(if s.eq_ignore_ascii_case("true") {
            true
        } else if s.eq_ignore_ascii_case("false") {
            false
        } else {
            anyhow::bail!(r#"Invalid bool value {s:?}, expected "True" or "False""#);
        })
    }
}

macro_rules! simple_ascom_param {
    ($($ty:ty),*) => {
        $(
            impl ASCOMParam for $ty {
                fn from_string(s: String) -> anyhow::Result<Self> {
                    Ok(s.parse()?)
                }
            }
        )*
    };
}

simple_ascom_param!(i32, u32, f64);

macro_rules! ASCOMEnumParam {
    ($(# $attr:tt)* $pub:vis enum $name:ident $variants:tt) => {
        impl $crate::server::ASCOMParam for $name {
            fn from_string(s: String) -> anyhow::Result<Self> {
                Ok(<Self as num_enum::TryFromPrimitive>::try_from_primitive(
                    $crate::server::ASCOMParam::from_string(s)?,
                )?)
            }
        }
    };
}
pub(crate) use ASCOMEnumParam;
