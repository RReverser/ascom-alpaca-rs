mod discovery;
pub use discovery::Client as DiscoveryClient;

mod transaction;
pub(crate) use transaction::*;

mod params;
pub(crate) use params::{opaque_params, ActionParams};

mod response;
pub(crate) use response::{OpaqueResponse, Response};

use crate::api::{ConfiguredDevice, DevicePath, ServerInfo};
use crate::response::ValueResponse;
use crate::{ASCOMError, ASCOMErrorCode, ASCOMResult, Devices};
use anyhow::Context;
use futures::TryFutureExt;
use mime::Mime;
use reqwest::header::CONTENT_TYPE;
use reqwest::{IntoUrl, RequestBuilder};
use serde::Serialize;
use std::fmt::Debug;
use std::net::SocketAddr;
use tracing::Instrument;

#[derive(Debug)]
pub(crate) struct DeviceClient {
    pub(crate) inner: RawClient,
    pub(crate) name: String,
    pub(crate) unique_id: String,
}

impl DeviceClient {
    pub(crate) async fn exec_action<Resp>(
        &self,
        action: &str,
        params: ActionParams<impl Debug + Serialize + Send>,
    ) -> ASCOMResult<Resp>
    where
        ASCOMResult<Resp>: Response,
    {
        self.inner
            .request::<ASCOMResult<Resp>>(action, params)
            .await
            .unwrap_or_else(|err| {
                Err(ASCOMError::new(
                    ASCOMErrorCode::UNSPECIFIED,
                    format!("{err:#}"),
                ))
            })
    }
}

#[derive(Debug)]
pub(crate) struct RawClient {
    pub(crate) inner: reqwest::Client,
    pub(crate) base_url: reqwest::Url,
    pub(crate) client_id: u32,
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
        path: &str,
        params: ActionParams<impl Debug + Serialize + Send>,
    ) -> anyhow::Result<Resp> {
        let request_transaction = RequestTransaction::new(self.client_id);

        let span = tracing::debug_span!(
            "Alpaca transaction",
            path,
            ?params,
            client_transaction_id = request_transaction.client_transaction_id,
            client_id = request_transaction.client_id,
        );

        async move {
            let mut request = self.inner.request(
                match params {
                    ActionParams::Get(_) => reqwest::Method::GET,
                    ActionParams::Put(_) => reqwest::Method::PUT,
                },
                self.base_url.join(path)?,
            );

            let add_params = match params {
                ActionParams::Get(_) => RequestBuilder::query,
                ActionParams::Put(_) => RequestBuilder::form,
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
            err.context(format!("Failed to send Alpaca request to {path}"))
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

#[derive(Debug)]
pub struct Client {
    inner: RawClient,
}

impl Client {
    pub fn new(base_url: impl IntoUrl) -> anyhow::Result<Self> {
        RawClient::new(base_url.into_url()?).map(|inner| Self { inner })
    }

    pub fn new_from_addr(addr: impl Into<SocketAddr>) -> anyhow::Result<Self> {
        Self::new(format!("http://{}/", addr.into()))
    }

    pub async fn get_devices(&self) -> anyhow::Result<Devices> {
        let mut devices = Devices::default();

        self.inner
            .request::<OpaqueResponse>(
                "management/v1/configureddevices",
                ActionParams::Get(opaque_params! {}),
            )
            .await?
            .try_as::<ValueResponse<Vec<ConfiguredDevice>>>()
            .context("Couldn't parse list of devices")?
            .into()
            .into_iter()
            .try_for_each(|device| {
                let device_client = DeviceClient {
                    inner: self.inner.join_url(&format!(
                        "api/v1/{device_type}/{device_number}/",
                        device_type = DevicePath(device.ty),
                        device_number = device.number
                    ))?,
                    name: device.name,
                    unique_id: device.unique_id,
                };

                device_client.add_to_as(&mut devices, device.ty);

                Ok::<_, anyhow::Error>(())
            })?;

        Ok(devices)
    }

    pub async fn get_server_info(&self) -> anyhow::Result<ServerInfo> {
        self.inner
            .request::<OpaqueResponse>(
                "management/v1/description",
                ActionParams::Get(opaque_params! {}),
            )
            .await?
            .try_as::<ValueResponse<ServerInfo>>()
            .map(ValueResponse::into)
            .context("Couldn't parse server info")
    }
}
