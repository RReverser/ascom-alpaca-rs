use super::ResponseWithTransaction;
use crate::client::ResponseTransaction;
use crate::response::ValueResponse;
use crate::{ASCOMError, ASCOMErrorCode, ASCOMResult};
use mime::Mime;
use serde::de::value::UnitDeserializer;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer};

trait FromJsonBytes: Sized {
    fn from_json_bytes(bytes: &[u8]) -> serde_json::Result<Self>;
}

impl<T: DeserializeOwned> FromJsonBytes for T {
    fn from_json_bytes(bytes: &[u8]) -> serde_json::Result<Self> {
        serde_json::from_slice(bytes)
    }
}

#[derive(Debug)]
struct Flattened<A, B>(pub(crate) A, pub(crate) B);

impl<A: FromJsonBytes, B: FromJsonBytes> FromJsonBytes for Flattened<A, B> {
    fn from_json_bytes(bytes: &[u8]) -> serde_json::Result<Self> {
        Ok(Self(A::from_json_bytes(bytes)?, B::from_json_bytes(bytes)?))
    }
}

pub(crate) trait Response: Sized {
    fn prepare_reqwest(request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        request
    }

    fn from_reqwest(mime_type: Mime, bytes: &[u8]) -> eyre::Result<ResponseWithTransaction<Self>>;
}

struct JsonResponse<T>(T);

impl<T: FromJsonBytes> Response for JsonResponse<T> {
    fn from_reqwest(mime_type: Mime, bytes: &[u8]) -> eyre::Result<ResponseWithTransaction<Self>> {
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
            <Flattened<ResponseTransaction, T>>::from_json_bytes(bytes)?;

        Ok(ResponseWithTransaction {
            transaction,
            response: Self(response),
        })
    }
}

impl<T: DeserializeOwned> Response for ASCOMResult<T> {
    fn from_reqwest(mime_type: Mime, bytes: &[u8]) -> eyre::Result<ResponseWithTransaction<Self>> {
        struct ParseResult<T>(eyre::Result<T>);

        impl<'de, T: Deserialize<'de>> Deserialize<'de> for ParseResult<T> {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                Ok(Self(
                    if size_of::<T>() == 0 {
                        // serde doesn't consider empty maps to be a valid source for the `()` type.
                        // We could tweak the type itself, but it's easier to just special-case empty types here.
                        T::deserialize(UnitDeserializer::new())
                    } else {
                        T::deserialize(deserializer)
                    }
                    .map_err(|err| eyre::eyre!("{err}")),
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
    fn from_reqwest(mime_type: Mime, bytes: &[u8]) -> eyre::Result<ResponseWithTransaction<Self>> {
        Ok(JsonResponse::from_reqwest(mime_type, bytes)?
            .map(|JsonResponse(value_response)| value_response))
    }
}
