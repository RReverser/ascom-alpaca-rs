use super::case_insensitive_str::CaseInsensitiveStr;
use super::Error;
use axum::extract::{FromRequest, Request};
use axum::response::IntoResponse;
use axum::Form;
use http::{Method, StatusCode};
use indexmap::IndexMap;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::fmt::Debug;
use std::hash::Hash;

#[derive(Debug, Deserialize)]
#[serde(transparent)]
#[serde(bound(deserialize = "Box<ParamStr>: serde::de::DeserializeOwned + Hash + Eq"))]
pub(crate) struct OpaqueParams<ParamStr: ?Sized>(IndexMap<Box<ParamStr>, String>);

#[derive(Debug)]
pub(crate) enum ActionParams {
    Get(OpaqueParams<CaseInsensitiveStr>),
    Put(OpaqueParams<str>),
}

impl<ParamStr: ?Sized + Hash + Eq + Debug> OpaqueParams<ParamStr>
where
    str: AsRef<ParamStr>,
{
    pub(crate) fn maybe_extract<T: DeserializeOwned>(
        &mut self,
        name: &'static str,
    ) -> super::Result<Option<T>> {
        self.0
            .swap_remove(name.as_ref())
            .map(|value| serde_plain::from_str(&value))
            .transpose()
            .map_err(|err| Error::BadParameter { name, err })
    }

    pub(crate) fn extract<T: DeserializeOwned>(&mut self, name: &'static str) -> super::Result<T> {
        self.maybe_extract(name)?
            .ok_or(Error::MissingParameter { name })
    }

    pub(crate) fn finish_extraction(self) {
        if !self.0.is_empty() {
            tracing::warn!("Unused parameters: {:?}", self.0.keys());
        }
    }
}

#[async_trait::async_trait]
impl<S: Send + Sync> FromRequest<S> for ActionParams {
    type Rejection = axum::response::Response;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
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
