use super::ResponseWithTransaction;
use crate::client::ResponseTransaction;
use crate::response::ValueResponse;
use crate::{ASCOMError, ASCOMErrorCode, ASCOMResult};
use mime::Mime;
use serde::de::value::UnitDeserializer;
use serde::de::DeserializeOwned;
use std::any::TypeId;

pub(crate) trait Response: Sized {
    fn prepare_reqwest(request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        request
    }

    fn from_reqwest(mime_type: Mime, bytes: &[u8]) -> eyre::Result<ResponseWithTransaction<Self>>;
}

impl ResponseTransaction {
    #[allow(clippy::needless_pass_by_value)]
    pub(crate) fn from_reqwest(mime_type: Mime, bytes: &[u8]) -> eyre::Result<Self> {
        eyre::ensure!(
            mime_type.essence_str() == mime::APPLICATION_JSON.as_ref(),
            "Expected JSON response, got {}",
            mime_type,
        );
        match mime_type.get_param(mime::CHARSET) {
            Some(mime::UTF_8) | None => {}
            Some(charset) => eyre::bail!("Unsupported charset {}", charset),
        }
        Ok(serde_json::from_slice(bytes)?)
    }
}

impl<T: 'static + DeserializeOwned> Response for ASCOMResult<T> {
    fn from_reqwest(mime_type: Mime, bytes: &[u8]) -> eyre::Result<ResponseWithTransaction<Self>> {
        let transaction = ResponseTransaction::from_reqwest(mime_type, bytes)?;
        let ascom_error = serde_json::from_slice::<ASCOMError>(bytes)?;

        Ok(ResponseWithTransaction {
            transaction,
            response: match ascom_error.code {
                ASCOMErrorCode::OK => Ok(if TypeId::of::<T>() == TypeId::of::<()>() {
                    // Specialization: avoid failure when trying to parse `()` from JSON object with no `Value`.
                    T::deserialize(UnitDeserializer::new())
                } else {
                    serde_json::from_slice::<ValueResponse<T>>(bytes)
                        .map(|value_response| value_response.value)
                }?),
                _ => Err(ascom_error),
            },
        })
    }
}

impl<T: DeserializeOwned> Response for ValueResponse<T> {
    fn from_reqwest(mime_type: Mime, bytes: &[u8]) -> eyre::Result<ResponseWithTransaction<Self>> {
        Ok(ResponseWithTransaction {
            transaction: ResponseTransaction::from_reqwest(mime_type, bytes)?,
            response: serde_json::from_slice(bytes)?,
        })
    }
}
