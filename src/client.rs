use crate::api::ConfiguredDevice;
use crate::api::ServerInfo;
use crate::params::OpaqueParams;
use crate::response::OpaqueResponse;
use crate::response::Response;
use crate::Devices;
use crate::{ASCOMError, ASCOMErrorCode, ASCOMResult};
use anyhow::Context;
use mime::Mime;
use reqwest::header::CONTENT_TYPE;
use reqwest::IntoUrl;
use std::net::SocketAddr;
use tracing::Instrument;
use crate::transaction::ClientRequestTransaction;
use crate::transaction::ClientRequestWithTransaction;
use futures::TryFutureExt;

#[derive(Debug)]
pub(crate) struct DeviceClient {
    pub(crate) inner: RawClient,
    pub(crate) unique_id: String,
}

impl DeviceClient {
    pub(crate) async fn exec_action<Resp>(
        &self,
        is_mut: bool,
        action: &str,
        params: OpaqueParams<str>,
    ) -> ASCOMResult<Resp>
    where
        ASCOMResult<Resp>: Response,
    {
        self.inner
            .request::<ASCOMResult<Resp>>(
                action,
                is_mut,
                params,
                |request| request,
            )
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
        is_mut: bool,
        params: OpaqueParams<str>,
        fill: impl FnOnce(reqwest::RequestBuilder) -> reqwest::RequestBuilder + Send,
    ) -> anyhow::Result<Resp> {
        let request_transaction = ClientRequestTransaction::new(self.client_id);

        let span = tracing::debug_span!(
            "Alpaca transaction",
            path,
            ?params,
            is_mut,
            ?request_transaction,
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

            let params = ClientRequestWithTransaction {
                transaction: request_transaction,
                params,
            };
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
            let (response_transaction, response) = Resp::from_reqwest(mime_type, bytes)?;

            tracing::debug!(
                server_transaction_id = response_transaction.server_transaction_id,
                "Received response",
            );

            match response_transaction.client_transaction_id {
                Some(received_client_transaction_id)
                    if received_client_transaction_id != request_transaction.client_transaction_id =>
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

        self.inner.request::<OpaqueResponse>(
            "management/v1/configureddevices",
            false,
            OpaqueParams::default(),
            |request| request,
        )
        .await?
        .try_as::<Vec<ConfiguredDevice>>()
        .context("Couldn't parse list of devices")?
        .into_iter()
        .try_for_each(|device| {
            let device_client = DeviceClient {
                unique_id: device.unique_id,
                inner: self.inner.join_url(&format!(
                    "api/v1/{device_type}/{device_number}/",
                    device_type = device.device_type,
                    device_number = device.device_number
                ))?
            };

            device_client.add_to_as(&mut devices, device.device_type);

            Ok::<_, anyhow::Error>(())
        })?;

        Ok(devices)
    }

    pub async fn get_server_info(&self) -> anyhow::Result<ServerInfo> {
        self.inner.request::<OpaqueResponse>(
            "management/v1/description",
            false,
            OpaqueParams::default(),
            |request| request,
        )
        .await?
        .try_as::<ServerInfo>()
        .context("Couldn't parse server info")
    }
}
