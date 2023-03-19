mod discovery;
pub use discovery::Server as DiscoveryServer;

mod transaction;
pub(crate) use transaction::*;

mod params;
pub(crate) use params::ActionParams;

mod response;
pub(crate) use response::{OpaqueResponse, Response};

use crate::api::{CargoServerInfo, ConfiguredDevice, DevicePath, ServerInfo};
use crate::discovery::DEFAULT_DISCOVERY_PORT;
use crate::Devices;
use axum::extract::Path;
use axum::routing::MethodFilter;
use axum::Router;
use futures::{StreamExt, TryFutureExt};
use net_literals::addr;
use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::Instrument;
use crate::response::ValueResponse;
#[cfg(feature = "camera")]
use crate::api::{Camera, DeviceType};

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

async fn server_handler<
    Resp: Response,
    RespFut: Future<Output = axum::response::Result<Resp>> + Send,
>(
    path: &str,
    mut raw_opaque_params: ActionParams,
    make_response: impl FnOnce(ActionParams) -> RespFut + Send,
) -> axum::response::Result<axum::response::Response> {
    let request_transaction = RequestTransaction::extract(&mut raw_opaque_params)
        .map_err(|err| (axum::http::StatusCode::BAD_REQUEST, format!("{err:#}")))?;
    let response_transaction = ResponseTransaction::new(request_transaction.client_transaction_id);

    let span = tracing::debug_span!(
        "Alpaca transaction",
        path,
        params = ?raw_opaque_params,
        client_id = request_transaction.client_id,
        client_transaction_id = request_transaction.client_transaction_id,
        server_transaction_id = response_transaction.server_transaction_id,
    );

    async move {
        let response = make_response(raw_opaque_params).await?;
        Ok(response.into_axum(response_transaction))
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
        let server_info = OpaqueResponse::new(ValueResponse::from(self.info));

        Router::new()
            .route(
                "/management/apiversions",
                axum::routing::get(|params| {
                    server_handler("/management/apiversions",  params, |_params| async move {
                        Ok(OpaqueResponse::new(ValueResponse::from([1_u32])))
                    })
                }),
            )
            .route("/management/v1/configureddevices", {
                let this = Arc::clone(&devices);

                axum::routing::get(|params| {
                    server_handler("/management/v1/configureddevices",  params, |_params| async move {
                        let devices = this.stream_configured().collect::<Vec<ConfiguredDevice>>().await;
                        Ok(OpaqueResponse::new(ValueResponse::from(devices)))
                    })
                })
            })
            .route("/management/v1/description",
                axum::routing::get(move |params| {
                    server_handler("/management/v1/serverinfo", params, |_params| async move {
                        Ok(server_info.clone())
                    })
                })
            )
            .route(
                "/api/v1/:device_type/:device_number/:action",
                axum::routing::on(
                    MethodFilter::GET | MethodFilter::PUT,
                    move |
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
                                && crate::api::ImageArrayResponse::is_accepted(&headers)
                            {
                                return server_handler(&format!("/api/v1/{device_type}/{device_number}/{action} with ImageBytes"), params, |_params| async move {
                                    Ok(<dyn Camera>::get_in(&devices, device_number)?.read().await.image_array().await)
                                }).await;
                            }
                        }

                        server_handler(&format!("/api/v1/{device_type}/{device_number}/{action}"),  params, |params| {
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
