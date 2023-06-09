use super::parse_flattened::Flattened;
use super::ResponseWithTransaction;
use crate::client::ResponseTransaction;
use crate::response::ValueResponse;
use crate::{ASCOMError, ASCOMErrorCode, ASCOMResult};
use bytes::Bytes;
use mime::Mime;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer};

pub(crate) trait Response: Sized {
    fn prepare_reqwest(request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        request
    }

    fn from_reqwest(mime_type: Mime, bytes: Bytes) -> eyre::Result<ResponseWithTransaction<Self>>;
}

struct JsonResponse<T>(T);

impl<T: DeserializeOwned> Response for JsonResponse<T> {
    fn from_reqwest(mime_type: Mime, bytes: Bytes) -> eyre::Result<ResponseWithTransaction<Self>> {
        eyre::ensure!(
            mime_type.essence_str() == mime::APPLICATION_JSON.as_ref(),
            "Expected JSON response, got {}",
            mime_type,
        );
        match mime_type.get_param(mime::CHARSET) {
            Some(mime::UTF_8) | None => {}
            Some(charset) => eyre::bail!("Unsupported charset {}", charset),
        };

        let Flattened(transaction, response) =
            serde_json::from_slice::<Flattened<ResponseTransaction, T>>(&bytes)?;

        Ok(ResponseWithTransaction {
            transaction,
            response: Self(response),
        })
    }
}

impl<T: DeserializeOwned> Response for ASCOMResult<T> {
    fn from_reqwest(mime_type: Mime, bytes: Bytes) -> eyre::Result<ResponseWithTransaction<Self>> {
        struct ParseResult<T>(eyre::Result<T>);

        impl<'de, T: Deserialize<'de>> Deserialize<'de> for ParseResult<T> {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                Ok(Self(
                    T::deserialize(deserializer).map_err(|err| eyre::eyre!("{err}")),
                ))
            }
        }

        JsonResponse::<Flattened<ASCOMError, ParseResult<T>>>::from_reqwest(mime_type, bytes)?
            .try_map(
                |JsonResponse(Flattened(ascom_error, ParseResult(parse_result)))| {
                    Ok(if ascom_error.code == ASCOMErrorCode::OK {
                        Ok(parse_result?)
                    } else {
                        Err(ascom_error)
                    })
                },
            )
    }
}

impl<T: DeserializeOwned> Response for ValueResponse<T> {
    fn from_reqwest(mime_type: Mime, bytes: Bytes) -> eyre::Result<ResponseWithTransaction<Self>> {
        Ok(JsonResponse::from_reqwest(mime_type, bytes)?
            .map(|JsonResponse(value_response)| value_response))
    }
}
