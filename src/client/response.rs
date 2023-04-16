use super::ResponseWithTransaction;
use crate::response::ValueResponse;
use crate::{ASCOMError, ASCOMErrorCode, ASCOMResult};
use bytes::Bytes;
use mime::Mime;
use serde::de::DeserializeOwned;

pub(crate) trait Response: Sized {
    fn prepare_reqwest(request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        request
    }

    fn from_reqwest(mime_type: Mime, bytes: Bytes)
        -> anyhow::Result<ResponseWithTransaction<Self>>;
}

struct JsonResponse<T>(T);

impl<T: 'static + DeserializeOwned> Response for JsonResponse<T> {
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

        Ok(ResponseWithTransaction {
            transaction: serde_json::from_slice(&bytes)?,
            response: Self(serde_json::from_slice(
                if std::any::TypeId::of::<T>() == std::any::TypeId::of::<()>() {
                    // workaround for serde expecting `null` for unit type, but we want to support & ignore arbitrary input
                    b"null"
                } else {
                    &bytes
                },
            )?),
        })
    }
}

impl<T: 'static + DeserializeOwned> Response for ASCOMResult<T> {
    fn from_reqwest(
        mime_type: Mime,
        bytes: Bytes,
    ) -> anyhow::Result<ResponseWithTransaction<Self>> {
        let ascom_error = serde_json::from_slice::<ASCOMError>(&bytes)?;
        if ascom_error.code == ASCOMErrorCode::OK {
            Ok(JsonResponse::from_reqwest(mime_type, bytes)?.map(|JsonResponse(value)| Ok(value)))
        } else {
            Ok(ResponseWithTransaction {
                transaction: serde_json::from_slice(&bytes)?,
                response: Err(ascom_error),
            })
        }
    }
}

impl<T: 'static + DeserializeOwned> Response for ValueResponse<T> {
    fn from_reqwest(
        mime_type: Mime,
        bytes: Bytes,
    ) -> anyhow::Result<ResponseWithTransaction<Self>> {
        Ok(JsonResponse::from_reqwest(mime_type, bytes)?
            .map(|JsonResponse(value_response)| value_response))
    }
}
