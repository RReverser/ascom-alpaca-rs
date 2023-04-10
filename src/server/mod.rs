mod discovery;
pub use discovery::Server as DiscoveryServer;

mod transaction;
pub(crate) use transaction::*;

mod params;
pub(crate) use params::ActionParams;

mod response;
pub(crate) use response::Response;

mod error;
pub(crate) use error::Error;

#[cfg(feature = "camera")]
use crate::api::{Camera, DeviceType};
use crate::api::{CargoServerInfo, DevicePath, ServerInfo};
use crate::discovery::DEFAULT_DISCOVERY_PORT;
use crate::response::ValueResponse;
use crate::Devices;
use axum::extract::Path;
use axum::http::Uri;
use axum::response::IntoResponse;
use axum::routing::MethodFilter;
use axum::Router;
use futures::TryFutureExt;
use net_literals::addr;
use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::Instrument;

#[derive(Debug)]
pub struct Server {
    pub devices: Devices,
    pub info: ServerInfo,
    pub listen_addr: SocketAddr,
    pub discovery_port: u16,
}

impl Default for Server {
    fn default() -> Self {
        Self {
            devices: Devices::default(),
            info: CargoServerInfo!(),
            listen_addr: addr!("[::]:0"),
            discovery_port: DEFAULT_DISCOVERY_PORT,
        }
    }
}

async fn server_handler<Resp: Response, RespFut: Future<Output = Resp> + Send>(
    uri: Uri,
    mut raw_opaque_params: ActionParams,
    make_response: impl FnOnce(ActionParams) -> RespFut + Send,
) -> axum::response::Response {
    let request_transaction = match RequestTransaction::extract(&mut raw_opaque_params) {
        Ok(transaction) => transaction,
        Err(err) => {
            return (axum::http::StatusCode::BAD_REQUEST, format!("{err:#}")).into_response();
        }
    };
    let response_transaction = ResponseTransaction::new(request_transaction.client_transaction_id);

    let span = tracing::debug_span!(
        "Alpaca transaction",
        path = uri.path(),
        params = ?raw_opaque_params,
        client_id = request_transaction.client_id,
        client_transaction_id = request_transaction.client_transaction_id,
        server_transaction_id = response_transaction.server_transaction_id,
    );

    async move {
        make_response(raw_opaque_params)
            .await
            .into_axum(response_transaction)
    }
    .instrument(span)
    .await
}

impl Server {
    pub async fn start_server(self) -> anyhow::Result<()> {
        let mut addr = self.listen_addr;

        tracing::debug!(%addr, "Binding Alpaca server");

        let server = axum::Server::try_bind(&addr)?.serve(
            self.into_router()
                // .layer(TraceLayer::new_for_http())
                .into_make_service(),
        );

        // The address can differ e.g. when using port 0 (auto-assigned).
        addr = server.local_addr();

        tracing::info!(%addr, "Bound Alpaca server");

        tracing::debug!("Starting Alpaca main and discovery servers");

        // Start the discovery server only once we ensured that the Alpaca server is bound to a port successfully.
        tokio::try_join!(
            server.map_err(Into::into),
            DiscoveryServer::new(addr.port()).start_server()
        )?;

        Ok(())
    }

    pub fn into_router(self) -> Router {
        let devices = Arc::new(self.devices);
        let server_info = Arc::new(self.info);

        Router::new()
            .route(
                "/management/apiversions",
                axum::routing::get(|uri, params| {
                    server_handler(uri,  params, |_params| async move {
                        ValueResponse::from([1_u32])
                    })
                }),
            )
            .route("/management/v1/configureddevices", {
                let this = Arc::clone(&devices);

                axum::routing::get(|uri, params| {
                    server_handler(uri,  params, |_params| async move {
                        let devices = this.iter_all().map(|(device, number)| device.to_configured_device(number)).collect::<Vec<_>>();
                        ValueResponse::from(devices)
                    })
                })
            })
            .route("/management/v1/description",
                axum::routing::get(move |uri, params| {
                    server_handler(uri, params, |_params| async move {
                        ValueResponse::from(Arc::clone(&server_info))
                    })
                })
            )
            .route(
                "/api/v1/:device_type/:device_number/:action",
                axum::routing::on(
                    MethodFilter::GET | MethodFilter::PUT,
                    move |
                        uri,
                        #[cfg_attr(not(feature = "camera"), allow(unused_mut))]
                        Path((DevicePath(device_type), device_number, mut action)): Path<(
                            DevicePath,
                            usize,
                            String,
                        )>,
                        #[cfg(feature = "camera")]
                        headers: axum::http::HeaderMap,
                        params: ActionParams
                    | async move {
                        #[cfg(feature = "camera")]
                        if device_type == DeviceType::Camera {
                            // imagearrayvariant is soft-deprecated; we should accept it but
                            // forward to the imagearray handler instead.
                            if action == "imagearrayvariant" {
                                action.truncate("imagearray".len());
                            }

                            if matches!(params, ActionParams::Get { .. })
                                && action == "imagearray"
                                && crate::api::ImageArray::is_accepted(&headers)
                            {
                                return server_handler(uri, params, |_params| async move {
                                    Ok::<_, Error>(crate::api::ImageBytesResponse(devices.get_for_server::<dyn Camera>(device_number)?.image_array().await?))
                                }).await;
                            }
                        }

                        server_handler(uri,  params, |params| {
                            devices.handle_action(
                                device_type,
                                device_number,
                                &action,
                                params,
                            )
                        }).await
                    }),
            )
    }
}
