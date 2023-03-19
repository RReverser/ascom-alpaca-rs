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
use crate::api::{CargoServerInfo, ConfiguredDevice, DevicePath, ServerInfo};
use crate::discovery::DEFAULT_DISCOVERY_PORT;
use crate::response::ValueResponse;
use crate::{ASCOMResult, Devices};
use axum::extract::Path;
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

async fn server_handler<Resp, RespFut: Future<Output = Result<Resp, Error>> + Send>(
    path: &str,
    mut raw_opaque_params: ActionParams,
    make_response: impl FnOnce(ActionParams) -> RespFut + Send,
) -> Result<axum::response::Response, (axum::http::StatusCode, String)>
where
    ASCOMResult<Resp>: Response,
{
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
        let ascom_result = match make_response(raw_opaque_params).await {
            Ok(response) => Ok(response),
            Err(Error::Ascom(err)) => Err(err),
            Err(Error::BadRequest(err)) => {
                return Err((axum::http::StatusCode::BAD_REQUEST, format!("{err:#}")));
            }
            Err(Error::NotFound(err)) => {
                return Err((axum::http::StatusCode::NOT_FOUND, format!("{err:#}")));
            }
        };
        Ok(ascom_result.into_axum(response_transaction))
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
                axum::routing::get(|params| {
                    server_handler("/management/apiversions",  params, |_params| async move {
                        Ok(ValueResponse::from([1_u32]))
                    })
                }),
            )
            .route("/management/v1/configureddevices", {
                let this = Arc::clone(&devices);

                axum::routing::get(|params| {
                    server_handler("/management/v1/configureddevices",  params, |_params| async move {
                        let devices = this.iter().collect::<Vec<ConfiguredDevice>>();
                        Ok(ValueResponse::from(devices))
                    })
                })
            })
            .route("/management/v1/description",
                axum::routing::get(move |params| {
                    server_handler("/management/v1/serverinfo", params, |_params| async move {
                        Ok(ValueResponse::from(Arc::clone(&server_info)))
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
                                    Ok(crate::api::ImageBytesResponse(<dyn Camera>::get_in(&devices, device_number)?.image_array().await?))
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
