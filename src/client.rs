use crate::api::ConfiguredDevice;
use crate::api::Device;
use crate::api::DeviceType;
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
use std::sync::Arc;
use tracing::Instrument;
use crate::transaction::ClientRequestTransaction;
use crate::transaction::ClientRequestWithTransaction;
use futures::TryFutureExt;

#[derive(Debug)]
pub(crate) struct Sender {
    pub(crate) client: Arc<Client>,
    pub(crate) unique_id: String,
    pub(crate) device_type: DeviceType,
    pub(crate) device_number: usize,
}

impl Sender {
    pub(crate) async fn exec_action<Resp>(
        &self,
        is_mut: bool,
        action: &str,
        params: OpaqueParams<str>,
    ) -> ASCOMResult<Resp>
    where
        ASCOMResult<Resp>: Response,
    {
        self.client
            .request::<ASCOMResult<Resp>>(
                &format!(
                    "api/v1/{device_type}/{device_number}/{action}",
                    device_type = self.device_type,
                    device_number = self.device_number
                ),
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
pub struct Client {
    pub(crate) inner: reqwest::Client,
    pub(crate) base_url: reqwest::Url,
    pub(crate) client_id: u32,
}

impl Client {
    pub fn new(base_url: impl IntoUrl) -> anyhow::Result<Arc<Self>> {
        let base_url = base_url.into_url()?;
        anyhow::ensure!(
            !base_url.cannot_be_a_base(),
            "{base_url} is not a valid base URL"
        );
        Ok(Arc::new(Self {
            inner: reqwest::Client::new(),
            base_url,
            client_id: rand::random(),
        }))
    }

    pub fn new_from_addr(addr: impl Into<SocketAddr>) -> anyhow::Result<Arc<Self>> {
        let addr = addr.into();
        Self::new(format!("http://{addr}/"))
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
        .context("Couldn't parse list of devices")?
        .into_iter()
        .for_each(|device| {
            devices.register::<dyn Device>(Sender {
                client: Arc::clone(self),
                unique_id: device.unique_id,
                device_type: device.device_type,
                device_number: device.device_number,
            });
        });

        Ok(devices)
    }

    pub async fn get_server_info(&self) -> anyhow::Result<ServerInfo> {
        self.request::<OpaqueResponse>(
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
