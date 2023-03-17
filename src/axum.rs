#[cfg(feature = "camera")]
use crate::api::Camera;
use crate::api::{CargoServerInfo, ConfiguredDevice, DevicePath, DeviceType, ServerInfo};
use crate::discovery::{DiscoveryServer, DEFAULT_DISCOVERY_PORT};
use crate::params::RawActionParams;
use crate::response::OpaqueResponse;
use crate::transaction::server_handler;
use crate::Devices;
use axum::extract::Path;
use axum::headers::{Header, HeaderName, HeaderValue};
use axum::http::HeaderMap;
use axum::routing::MethodFilter;
use axum::Router;
use futures::{StreamExt, TryFutureExt};
use mediatype::MediaTypeList;
use net_literals::addr;
use serde::Serialize;
use std::net::SocketAddr;
use std::sync::Arc;

const MEDIA_TYPE_IMAGE_BYTES: mediatype::MediaType<'static> = mediatype::MediaType::new(
    mediatype::names::APPLICATION,
    mediatype::Name::new_unchecked("imagebytes"),
);

// A hack until TypedHeader supports Accept natively.
#[derive(Default)]
struct AcceptsImageBytes {
    accepts: bool,
}

impl Header for AcceptsImageBytes {
    fn name() -> &'static HeaderName {
        static ACCEPT: HeaderName = HeaderName::from_static("accept");
        &ACCEPT
    }

    fn decode<'value, I>(values: &mut I) -> Result<Self, axum::headers::Error>
    where
        Self: Sized,
        I: Iterator<Item = &'value HeaderValue>,
    {
        let mut accepts = false;
        for value in values {
            for media_type in MediaTypeList::new(
                value
                    .to_str()
                    .map_err(|_err| axum::headers::Error::invalid())?,
            ) {
                let media_type = media_type.map_err(|_err| axum::headers::Error::invalid())?;
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

#[derive(Debug, Serialize)]
struct ServerInfoValue {
    #[serde(rename = "Value")]
    server_info: ServerInfo,
}

#[derive(Debug)]
pub struct DevicesServer {
    pub devices: Devices,
    pub info: ServerInfo,
    pub listen_addr: SocketAddr,
    pub discovery_port: u16,
}

impl Default for DevicesServer {
    fn default() -> Self {
        Self {
            devices: Devices::default(),
            info: CargoServerInfo!(),
            listen_addr: addr!("[::]:0"),
            discovery_port: DEFAULT_DISCOVERY_PORT,
        }
    }
}

impl DevicesServer {
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
                axum::routing::get(|params: RawActionParams| {
                    server_handler("/management/apiversions",  params, |_params| async move {
                        Ok(OpaqueResponse::new([1_u32]))
                    })
                }),
            )
            .route("/management/v1/configureddevices", {
                let this = Arc::clone(&devices);

                axum::routing::get(|params: RawActionParams| {
                    server_handler("/management/v1/configureddevices",  params, |_params| async move {
                        let devices = this.stream_configured().collect::<Vec<ConfiguredDevice>>().await;
                        Ok(OpaqueResponse::new(devices))
                    })
                })
            })
            .route("/management/v1/description",
                axum::routing::get(move |params: RawActionParams| {
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
                          Path((DevicePath(device_type), device_number, mut action)): Path<(
                        DevicePath,
                        usize,
                        String,
                    )>,
                          headers: HeaderMap<HeaderValue>,
                          params: RawActionParams| {
                        async move {
                            #[cfg(feature = "camera")]
                            if device_type == DeviceType::Camera {
                                // imagearrayvariant is soft-deprecated; we should accept it but
                                // forward to the imagearray handler instead.
                                if action == "imagearrayvariant" {
                                    action.truncate("imagearray".len());
                                }

                                if matches!(params, RawActionParams::Get { .. })
                                    && device_type == DeviceType::Camera
                                    && action == "imagearray"
                                    && AcceptsImageBytes::decode(&mut headers.get_all(AcceptsImageBytes::name()).iter())
                                        .map_or(false, |h| h.accepts)
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
                        }
                    }),
            )
    }
}
