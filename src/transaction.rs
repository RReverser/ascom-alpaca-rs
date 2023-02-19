use crate::api::ConfiguredDevice;
use crate::client::Sender;
use crate::params::OpaqueParams;
use crate::response::OpaqueResponse;
use crate::{ASCOMError, ASCOMErrorCode, ASCOMResult, Devices};
use anyhow::Context;
use axum::response::IntoResponse;
use bytes::Bytes;
use futures::TryFutureExt;
use mime::Mime;
use reqwest::header::CONTENT_TYPE;
use serde::Serialize;
use std::future::Future;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use tracing::Instrument;

macro_rules! auto_increment {
    () => {{
        static COUNTER: AtomicU32 = AtomicU32::new(0);
        COUNTER.fetch_add(1, Ordering::Relaxed)
    }};
}

#[derive(Serialize)]
pub(crate) struct ServerResponseTransaction {
    pub(crate) client_transaction_id: Option<u32>,
    pub(crate) server_transaction_id: u32,
}

pub(crate) struct ClientResponseTransaction {
    pub(crate) client_transaction_id: Option<u32>,
    pub(crate) server_transaction_id: Option<u32>,
}

pub(crate) trait Response: Sized {
    fn into_axum(self, transaction: ServerResponseTransaction) -> axum::response::Response;
    fn from_reqwest(
        mime_type: Mime,
        bytes: Bytes,
    ) -> anyhow::Result<(ClientResponseTransaction, Self)>;
}

pub(crate) fn server_handler<
    Resp: Response,
    RespFut: Future<Output = axum::response::Result<Resp>> + Send,
>(
    path: &str,
    is_mut: bool,
    mut raw_opaque_params: OpaqueParams,
    make_response: impl FnOnce(OpaqueParams) -> RespFut + Send,
) -> impl Future<Output = axum::response::Result<axum::response::Response>> {
    let [client_id, client_transaction_id] = ["ClientID", "ClientTransactionID"].map(|name| {
        raw_opaque_params.maybe_extract(name).unwrap_or_else(|err| {
            tracing::warn!(%err, "Ignoring invalid {name}");
            None
        })
    });
    let server_transaction_id = auto_increment!();

    let span = tracing::debug_span!(
        "Alpaca transaction",
        path,
        params = ?raw_opaque_params,
        is_mut,
        client_id,
        client_transaction_id,
        server_transaction_id,
    );

    async move {
        let response = make_response(raw_opaque_params).await?;
        Ok(response.into_axum(ServerResponseTransaction {
            client_transaction_id,
            server_transaction_id,
        }))
    }
    .instrument(span)
}

#[derive(Debug)]
pub struct Client {
    inner: reqwest::Client,
    base_url: reqwest::Url,
    client_id: u32,
}

impl Client {
    pub fn new(base_url: reqwest::Url, client_id: u32) -> anyhow::Result<Arc<Self>> {
        anyhow::ensure!(
            !base_url.cannot_be_a_base(),
            "{base_url} is not a valid base URL"
        );
        Ok(Arc::new(Self {
            inner: reqwest::Client::new(),
            base_url,
            client_id,
        }))
    }

    pub(crate) async fn request<Resp: Response>(
        &self,
        path: &str,
        is_mut: bool,
        mut params: OpaqueParams,
        fill: impl FnOnce(reqwest::RequestBuilder) -> reqwest::RequestBuilder + Send,
    ) -> anyhow::Result<Resp> {
        let client_transaction_id = auto_increment!();

        let span = tracing::debug_span!(
            "Alpaca transaction",
            path,
            ?params,
            is_mut,
            client_id = self.client_id,
            client_transaction_id,
        );

        async move {
            let mut request = self.inner.request(
                if is_mut {
                    reqwest::Method::PUT
                } else {
                    reqwest::Method::GET
                },
                self.base_url.join(path)?,
            );

            params.insert("ClientID", self.client_id);
            params.insert("ClientTransactionID", client_transaction_id);
            request = if is_mut {
                request.form(&params)
            } else {
                request.query(&params)
            };

            request = fill(request);

            let response = request.send().await?.error_for_status()?;
            let mime_type = response
                .headers()
                .get(CONTENT_TYPE)
                .context("Missing Content-Type header")?
                .to_str()?
                .parse::<Mime>()?;
            let bytes = response.bytes().await?;
            let (transaction, response) = Resp::from_reqwest(mime_type, bytes)?;

            tracing::debug!(
                server_transaction_id = transaction.server_transaction_id,
                "Received response",
            );

            match transaction.client_transaction_id {
                Some(received_client_transaction_id)
                    if received_client_transaction_id != client_transaction_id =>
                {
                    tracing::warn!(
                        sent = client_transaction_id,
                        received = received_client_transaction_id,
                        "ClientTransactionID mismatch",
                    );
                }
                _ => {}
            }

            Ok::<_, anyhow::Error>(response)
        }
        .map_err(|err| {
            tracing::error!(%err, "Alpaca request failed");
            err.context(format!("Failed to send Alpaca request to {path}"))
        })
        .instrument(span)
        .await
    }

    pub async fn get_devices(self: &Arc<Self>) -> anyhow::Result<Devices> {
        let mut devices = Devices::default();

        self.request::<OpaqueResponse>(
            "management/v1/configureddevices",
            false,
            OpaqueParams::default(),
            |request| request,
        )
        .await?
        .try_as::<Vec<ConfiguredDevice>>()
        .map_err(|err| {
            tracing::error!(%err, "Couldn't parse list of devices");
            ASCOMError::new(ASCOMErrorCode::UNSPECIFIED, err.to_string())
        })?
        .into_iter()
        .try_for_each(|device| {
            let sender = Sender {
                client: Arc::clone(self),
                unique_id: device.unique_id,
                device_number: device.device_number,
            };
            sender.add_as(&device.device_type, &mut devices)
        })?;

        Ok(devices)
    }
}

impl Response for OpaqueResponse {
    fn into_axum(self, transaction: ServerResponseTransaction) -> axum::response::Response {
        #[derive(Serialize)]
        struct ResponseWithTransaction {
            #[serde(flatten)]
            transaction: ServerResponseTransaction,
            #[serde(flatten)]
            response: OpaqueResponse,
        }

        axum::response::Json(ResponseWithTransaction {
            transaction,
            response: self,
        })
        .into_response()
    }

    fn from_reqwest(
        mime_type: Mime,
        bytes: Bytes,
    ) -> anyhow::Result<(ClientResponseTransaction, Self)> {
        anyhow::ensure!(
            mime_type.essence_str() == mime::APPLICATION_JSON.as_ref(),
            "Expected JSON response, got {mime_type}"
        );
        match mime_type.get_param(mime::CHARSET) {
            Some(mime::UTF_8) | None => {}
            Some(charset) => anyhow::bail!("Unsupported charset {charset}"),
        };

        let mut opaque_response = serde_json::from_slice::<Self>(&bytes)?;
        let [client_transaction_id, server_transaction_id] =
            ["ClientTransactionID", "ServerTransactionID"].map(|name| {
                opaque_response.0.remove(name).and_then(|value| {
                    serde_json::from_value(value)
                        .map_err(|err| {
                            tracing::warn!(%err, "Ignoring invalid {name}");
                        })
                        .ok()
                })
            });

        Ok((
            ClientResponseTransaction {
                client_transaction_id,
                server_transaction_id,
            },
            opaque_response,
        ))
    }
}

impl Response for ASCOMResult<OpaqueResponse> {
    fn into_axum(self, transaction: ServerResponseTransaction) -> axum::response::Response {
        self.unwrap_or_else(|err| {
            tracing::error!(%err, "Alpaca method returned an error");
            OpaqueResponse::new(err)
        })
        .into_axum(transaction)
    }

    fn from_reqwest(
        mime_type: Mime,
        bytes: Bytes,
    ) -> anyhow::Result<(ClientResponseTransaction, Self)> {
        let (transaction, response) = OpaqueResponse::from_reqwest(mime_type, bytes)?;

        Ok((
            transaction,
            if response.0.contains_key("ErrorNumber") {
                Err(response.try_as::<ASCOMError>().unwrap_or_else(|err| {
                    ASCOMError::new(
                        ASCOMErrorCode::UNSPECIFIED,
                        format!("Server returned an error but it couldn't be parsed: {err}"),
                    )
                }))
            } else {
                Ok(response)
            },
        ))
    }
}
