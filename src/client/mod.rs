#[cfg(feature = "criterion")]
mod benches;
#[cfg(feature = "criterion")]
pub use benches::benches;

mod discovery;
pub use discovery::{BoundClient as BoundDiscoveryClient, Client as DiscoveryClient};

mod transaction;
pub(crate) use transaction::*;

mod response;
pub(crate) use response::Response;

use crate::api::{ConfiguredDevice, DevicePath, FallibleDeviceType, ServerInfo, TypedDevice};
use crate::params::{Action, ActionParams, Method};
use crate::response::ValueResponse;
use crate::{ASCOMError, ASCOMResult};
use eyre::ContextCompat;
use mime::Mime;
use reqwest::header::CONTENT_TYPE;
use reqwest::{IntoUrl, RequestBuilder};
use serde::Serialize;
use std::fmt::Debug;
use std::net::SocketAddr;
use std::num::NonZeroU32;
use std::sync::{Arc, LazyLock};
use tracing::Instrument;

#[derive(Debug)]
pub(crate) struct RawDeviceClient {
    pub(crate) inner: RawClient,
    pub(crate) name: String,
    pub(crate) unique_id: String,
}

impl RawDeviceClient {
    pub(crate) async fn exec_action<Resp>(&self, action: impl Action) -> ASCOMResult<Resp>
    where
        ASCOMResult<Resp>: Response,
    {
        self.inner
            .request::<ASCOMResult<Resp>>(action.into_parts())
            .await
            .unwrap_or_else(|err| Err(ASCOMError::unspecified(err)))
    }
}

pub(crate) static REQWEST: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .user_agent("ascom-alpaca-rs")
        .build()
        .expect("failed to create reqwest client")
});

#[derive(Clone, custom_debug::Debug)]
pub(crate) struct RawClient {
    #[debug(format = r#""{}""#)]
    pub(crate) base_url: reqwest::Url,
    pub(crate) client_id: NonZeroU32,
}

impl RawClient {
    pub(crate) fn new(base_url: reqwest::Url) -> eyre::Result<Self> {
        eyre::ensure!(
            !base_url.cannot_be_a_base(),
            "{} is not a valid base URL",
            base_url,
        );
        Ok(Self {
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
        }: ActionParams<impl Serialize + Send>,
    ) -> eyre::Result<Resp> {
        let request_transaction = RequestTransaction::new(self.client_id);

        let span = tracing::error_span!(
            "Alpaca transaction",
            action,
            client_transaction_id = request_transaction.client_transaction_id,
            client_id = request_transaction.client_id,
        );

        async move {
            tracing::debug!(?method, params = ?serdebug::debug(&params), base_url = %self.base_url, "Sending request");

            let mut request = REQWEST.request(method.into(), self.base_url.join(action)?);

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
            } = Resp::from_reqwest(mime_type, &bytes)?;

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

            Ok::<_, eyre::Error>(response)
        }
        .instrument(span)
        .await
    }

    pub(crate) fn join_url(&self, path: &str) -> eyre::Result<Self> {
        Ok(Self {
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
    pub fn new(base_url: impl IntoUrl) -> eyre::Result<Self> {
        RawClient::new(base_url.into_url()?).map(|inner| Self { inner })
    }

    /// Create a new client with given server address.
    pub fn new_from_addr(addr: impl Into<SocketAddr>) -> Self {
        Self::new(format!("http://{}/", addr.into()))
            .expect("creating client from an address should always succeed")
    }

    /// Get a list of all devices registered on the server.
    pub async fn get_devices(&self) -> eyre::Result<impl Iterator<Item = TypedDevice>> {
        let api_client = self.inner.join_url("api/v1/")?;

        Ok(self
            .inner
            .request::<ValueResponse<Vec<ConfiguredDevice<FallibleDeviceType>>>>(ActionParams {
                action: "management/v1/configureddevices",
                method: Method::Get,
                params: (),
            })
            .await?
            .value
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
    pub async fn get_server_info(&self) -> eyre::Result<ServerInfo> {
        self.inner
            .request::<ValueResponse<ServerInfo>>(ActionParams {
                action: "management/v1/description",
                method: Method::Get,
                params: (),
            })
            .await
            .map(|value_response| value_response.value)
    }
}
