mod discovery;
pub use discovery::Client as DiscoveryClient;

mod transaction;
pub(crate) use transaction::*;

mod params;
pub(crate) use params::{ActionParams, Method};

mod response;
pub(crate) use response::Response;

mod parse_flattened;

use crate::api::{ConfiguredDevice, DevicePath, FallibleDeviceType, ServerInfo, TypedDevice};
use crate::response::ValueResponse;
use crate::{ASCOMError, ASCOMErrorCode, ASCOMResult};
use anyhow::Context;
use futures::TryFutureExt;
use mime::Mime;
use reqwest::header::CONTENT_TYPE;
use reqwest::{IntoUrl, RequestBuilder};
use serde::Serialize;
use std::fmt::Debug;
use std::net::SocketAddr;
use std::num::NonZeroU32;
use std::sync::Arc;
use tracing::Instrument;

#[derive(Debug)]
pub(crate) struct RawDeviceClient {
    pub(crate) inner: RawClient,
    pub(crate) name: String,
    pub(crate) unique_id: String,
}

impl RawDeviceClient {
    pub(crate) async fn exec_action<Resp>(
        &self,
        action_params: ActionParams<impl Debug + Serialize + Send>,
    ) -> ASCOMResult<Resp>
    where
        ASCOMResult<Resp>: Response,
    {
        self.inner
            .request::<ASCOMResult<Resp>>(action_params)
            .await
            .unwrap_or_else(|err| {
                Err(ASCOMError::new(
                    ASCOMErrorCode::UNSPECIFIED,
                    format!("{err:#}"),
                ))
            })
    }
}

#[derive(Clone, custom_debug::Debug)]
pub(crate) struct RawClient {
    #[debug(skip)]
    pub(crate) inner: reqwest::Client,
    #[debug(format = r#""{}""#)]
    pub(crate) base_url: reqwest::Url,
    pub(crate) client_id: NonZeroU32,
}

impl RawClient {
    pub(crate) fn new(base_url: reqwest::Url) -> anyhow::Result<Self> {
        anyhow::ensure!(
            !base_url.cannot_be_a_base(),
            "{base_url} is not a valid base URL"
        );
        Ok(Self {
            inner: reqwest::Client::new(),
            base_url,
            client_id: rand::random(),
        })
    }

    pub(crate) async fn request<Resp: Response>(
        &self,
        ActionParams {
            action,
            method,
            params,
        }: ActionParams<impl Debug + Serialize + Send>,
    ) -> anyhow::Result<Resp> {
        let request_transaction = RequestTransaction::new(self.client_id);

        let span = tracing::debug_span!(
            "Alpaca transaction",
            action,
            ?params,
            base_url = %self.base_url,
            client_transaction_id = request_transaction.client_transaction_id,
            client_id = request_transaction.client_id,
        );

        async move {
            let mut request = self
                .inner
                .request(method.into(), self.base_url.join(action)?);

            let add_params = match method {
                Method::Get => RequestBuilder::query,
                Method::Put => RequestBuilder::form,
            };
            request = add_params(
                request,
                &RequestWithTransaction {
                    transaction: request_transaction,
                    params,
                },
            );

            request = Resp::prepare_reqwest(request);

            let response = request.send().await?.error_for_status()?;
            let mime_type = response
                .headers()
                .get(CONTENT_TYPE)
                .context("Missing Content-Type header")?
                .to_str()?
                .parse::<Mime>()?;
            let bytes = response.bytes().await?;
            let ResponseWithTransaction {
                transaction: response_transaction,
                response,
            } = Resp::from_reqwest(mime_type, bytes)?;

            tracing::debug!(
                server_transaction_id = response_transaction.server_transaction_id,
                "Received response",
            );

            match response_transaction.client_transaction_id {
                Some(received_client_transaction_id)
                    if received_client_transaction_id
                        != request_transaction.client_transaction_id =>
                {
                    tracing::warn!(
                        sent = request_transaction.client_transaction_id,
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
            err.context(format!(
                "Failed to send Alpaca request to {action} on {}",
                self.base_url
            ))
        })
        .instrument(span)
        .await
    }

    pub(crate) fn join_url(&self, path: &str) -> anyhow::Result<Self> {
        Ok(Self {
            inner: self.inner.clone(),
            base_url: self.base_url.join(path)?,
            client_id: self.client_id,
        })
    }
}

/// Alpaca client.
#[derive(Debug)]
pub struct Client {
    inner: RawClient,
}

impl Client {
    /// Create a new client with given server URL.
    pub fn new(base_url: impl IntoUrl) -> anyhow::Result<Self> {
        RawClient::new(base_url.into_url()?).map(|inner| Self { inner })
    }

    /// Create a new client with given server address.
    pub fn new_from_addr(addr: impl Into<SocketAddr>) -> anyhow::Result<Self> {
        Self::new(format!("http://{}/", addr.into()))
    }

    /// Get a list of all devices registered on the server.
    pub async fn get_devices(&self) -> anyhow::Result<impl Iterator<Item = TypedDevice>> {
        let api_client = self.inner.join_url("api/v1/")?;

        Ok(self
            .inner
            .request::<ValueResponse<Vec<ConfiguredDevice<FallibleDeviceType>>>>(ActionParams {
                action: "management/v1/configureddevices",
                method: Method::Get,
                params: (),
            })
            .await?
            .into_inner()
            .into_iter()
            .filter_map(move |device| match device.ty.0 {
                Ok(device_type) => Some(
                    Arc::new(RawDeviceClient {
                        inner: api_client
                            .join_url(&format!(
                                "{device_type}/{device_number}/",
                                device_type = DevicePath(device_type),
                                device_number = device.number
                            ))
                            .expect("internal error: failed to join device URL"),
                        name: device.name,
                        unique_id: device.unique_id,
                    })
                    .into_typed_client(device_type),
                ),
                Err(_) => {
                    tracing::warn!(?device, "Skipping device with unsupported type");
                    None
                }
            }))
    }

    /// Get general server information.
    pub async fn get_server_info(&self) -> anyhow::Result<ServerInfo> {
        Ok(self
            .inner
            .request::<ValueResponse<ServerInfo>>(ActionParams {
                action: "management/v1/description",
                method: Method::Get,
                params: (),
            })
            .await?
            .into_inner())
    }
}
