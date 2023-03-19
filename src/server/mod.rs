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
use serde::Serialize;
use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::Instrument;

#[cfg(all(feature = "camera", target_endian = "little"))]
mod image_bytes {
    use axum::headers::{Error, Header, HeaderMap, HeaderName, HeaderValue};
    use mediatype::{MediaType, MediaTypeList};

    const MEDIA_TYPE_IMAGE_BYTES: MediaType<'static> = MediaType::new(
        mediatype::names::APPLICATION,
        mediatype::Name::new_unchecked("imagebytes"),
    );

    // A hack until TypedHeader supports Accept natively.
    #[derive(Default)]
    pub(super) struct AcceptsImageBytes {
        accepts: bool,
    }

    impl AcceptsImageBytes {
        pub(super) fn extract(headers: &HeaderMap) -> bool {
            Self::decode(&mut headers.get_all(HeaderName::from_static("accept")).iter())
                .unwrap_or_default()
                .accepts
        }
    }

    impl Header for AcceptsImageBytes {
        fn name() -> &'static HeaderName {
            static ACCEPT: HeaderName = HeaderName::from_static("accept");
            &ACCEPT
        }

        fn decode<'value, I>(values: &mut I) -> Result<Self, Error>
        where
            Self: Sized,
            I: Iterator<Item = &'value HeaderValue>,
        {
            let mut accepts = false;
            for value in values {
                for media_type in
                    MediaTypeList::new(value.to_str().map_err(|_err| Error::invalid())?)
                {
                    let media_type = media_type.map_err(|_err| Error::invalid())?;
                    if media_type.essence() == MEDIA_TYPE_IMAGE_BYTES {
                        accepts = true;
                        break;
                    }
                }
            }
            Ok(Self { accepts })
        }

        fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
            values.extend(std::iter::once(HeaderValue::from_static(if self.accepts {
                "application/imagebytes"
            } else {
                "*/*"
            })));
        }
    }
}

#[cfg(all(feature = "camera", target_endian = "little"))]
use {
    crate::api::{Camera, DeviceType},
    axum::headers::{HeaderMap, HeaderValue},
    image_bytes::AcceptsImageBytes,
};

#[derive(Debug, Serialize)]
struct ServerInfoValue {
    #[serde(rename = "Value")]
    server_info: ServerInfo,
}

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
        let server_info = OpaqueResponse::new(ServerInfoValue {
            server_info: self.info,
        });

        Router::new()
            .route(
                "/management/apiversions",
                axum::routing::get(|params| {
                    server_handler("/management/apiversions",  params, |_params| async move {
                        Ok(OpaqueResponse::new([1_u32]))
                    })
                }),
            )
            .route("/management/v1/configureddevices", {
                let this = Arc::clone(&devices);

                axum::routing::get(|params| {
                    server_handler("/management/v1/configureddevices",  params, |_params| async move {
                        let devices = this.stream_configured().collect::<Vec<ConfiguredDevice>>().await;
                        Ok(OpaqueResponse::new(devices))
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
                        #[cfg_attr(not(all(feature = "camera", target_endian = "little")), allow(unused_mut))]
                        Path((DevicePath(device_type), device_number, mut action)): Path<(
                            DevicePath,
                            usize,
                            String,
                        )>,
                        #[cfg(all(feature = "camera", target_endian = "little"))]
                        headers: HeaderMap<HeaderValue>,
                        params: ActionParams
                    | async move {
                        #[cfg(all(feature = "camera", target_endian = "little"))]
                        if device_type == DeviceType::Camera {
                            // imagearrayvariant is soft-deprecated; we should accept it but
                            // forward to the imagearray handler instead.
                            if action == "imagearrayvariant" {
                                action.truncate("imagearray".len());
                            }

                            if matches!(params, ActionParams::Get { .. })
                                && device_type == DeviceType::Camera
                                && action == "imagearray"
                                && AcceptsImageBytes::extract(&headers)
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
