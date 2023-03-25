use super::ResponseWithTransaction;
use crate::{ASCOMError, ASCOMResult};
use anyhow::Context;
use bytes::Bytes;
use mime::Mime;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(transparent)]
pub(crate) struct OpaqueResponse(serde_json::Map<String, serde_json::Value>);

impl OpaqueResponse {
    pub(crate) fn maybe_extract<T: DeserializeOwned>(
        &mut self,
        name: &str,
    ) -> anyhow::Result<Option<T>> {
        self.0
            .remove(name)
            .map(serde_json::from_value)
            .transpose()
            .with_context(|| format!("couldn't parse {name}"))
    }

    pub(crate) fn extract<T: DeserializeOwned>(&mut self, name: &str) -> anyhow::Result<T> {
        self.maybe_extract(name)?
            .ok_or_else(|| anyhow::anyhow!("Missing parameter {name}"))
    }

    pub(crate) fn try_as<T: DeserializeOwned>(self) -> serde_json::Result<T> {
        serde_json::from_value(serde_json::Value::Object(self.0))
    }
}

pub(crate) trait Response: Sized {
    fn prepare_reqwest(request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        request
    }

    fn from_reqwest(mime_type: Mime, bytes: Bytes)
        -> anyhow::Result<ResponseWithTransaction<Self>>;
}

impl Response for OpaqueResponse {
    fn from_reqwest(
        mime_type: Mime,
        bytes: Bytes,
    ) -> anyhow::Result<ResponseWithTransaction<Self>> {
        anyhow::ensure!(
            mime_type.essence_str() == mime::APPLICATION_JSON.as_ref(),
            "Expected JSON response, got {mime_type}"
        );
        match mime_type.get_param(mime::CHARSET) {
            Some(mime::UTF_8) | None => {}
            Some(charset) => anyhow::bail!("Unsupported charset {charset}"),
        };

        serde_json::from_slice::<Self>(&bytes)?.try_into()
    }
}

impl Response for ASCOMResult<OpaqueResponse> {
    fn from_reqwest(
        mime_type: Mime,
        bytes: Bytes,
    ) -> anyhow::Result<ResponseWithTransaction<Self>> {
        Ok(
            OpaqueResponse::from_reqwest(mime_type, bytes)?.map(|mut response| {
                let status = ASCOMError::extract_status(&mut response);
                status.map(|_| response)
            }),
        )
    }
}
