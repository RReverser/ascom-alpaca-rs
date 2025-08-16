use crate::api::DevicePath;

mod discovery;
pub use discovery::{BoundServer as BoundDiscoveryServer, Server as DiscoveryServer};

mod transaction;
pub(crate) use transaction::*;

mod case_insensitive_str;

mod params;
pub(crate) use params::ActionParams;

mod response;

mod error;
pub(crate) use error::{Error, Result};

#[cfg(feature = "camera")]
use crate::api::Camera;
use crate::api::{CargoServerInfo, DeviceType, ServerInfo};
use crate::discovery::DEFAULT_DISCOVERY_PORT;
use crate::response::ValueResponse;
use crate::Devices;
use axum::extract::{FromRequest, Path, Request};
use axum::response::{Html, IntoResponse};
use axum::Router;
use futures::future::{BoxFuture, Future, FutureExt};
use http::StatusCode;
use net_literals::addr;
use sailfish::TemplateOnce;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::Instrument;

/// The Alpaca server.
#[derive(Debug)]
pub struct Server {
    /// Registered devices.
    pub devices: Devices,
    /// General server information.
    pub info: ServerInfo,
    /// Address for the server to listen on.
    pub listen_addr: SocketAddr,
    /// Port for the discovery server to listen on.
    pub discovery_port: u16,
}

impl Server {
    pub const fn default() -> Self {
        Self {
            devices: Devices::default(),
            info: CargoServerInfo!(),
            listen_addr: addr!("[::]:0"),
            discovery_port: DEFAULT_DISCOVERY_PORT,
        }
    }
}

impl Default for Server {
    fn default() -> Self {
        // Invoke the inherent const implementation.
        Self::default()
    }
}

struct ServerHandler {
    path: String,
    params: ActionParams,
}

impl<S: Send + Sync> FromRequest<S> for ServerHandler {
    type Rejection = axum::response::Response;

    async fn from_request(req: Request, state: &S) -> std::result::Result<Self, Self::Rejection> {
        let path = req.uri().path().to_owned();
        let params = ActionParams::from_request(req, state).await?;
        Ok(Self { path, params })
    }
}

impl ServerHandler {
    async fn exec<Output, RespFut: Future<Output = Output>>(
        mut self,
        make_response: impl FnOnce(ActionParams) -> RespFut,
    ) -> axum::response::Result<axum::response::Response>
    where
        ResponseWithTransaction<Output>: IntoResponse,
    {
        let request_transaction = RequestTransaction::extract(&mut self.params)?;
        let response_transaction =
            ResponseTransaction::new(request_transaction.client_transaction_id);

        let span = tracing::error_span!(
            "handle_alpaca_request",
            path = self.path,
            client_id = request_transaction.client_id,
            client_transaction_id = request_transaction.client_transaction_id,
            server_transaction_id = response_transaction.server_transaction_id,
        );

        Ok(async move {
            tracing::debug!(params = ?self.params, "Received request");

            ResponseWithTransaction {
                transaction: response_transaction,
                response: make_response(self.params).await,
            }
        }
        .instrument(span)
        .await
        .into_response())
    }
}

/// Alpaca servers bound to their respective ports and ready to listen.
#[derive(derive_more::Debug)]
pub struct BoundServer {
    // Axum types are a bit complicated, so just Box it for now.
    #[debug(skip)]
    axum: BoxFuture<'static, eyre::Result<std::convert::Infallible>>,
    axum_listen_addr: SocketAddr,
    discovery: BoundDiscoveryServer,
}

impl BoundServer {
    /// Returns the address the main Alpaca server is listening on.
    #[allow(clippy::missing_const_for_fn)] // we don't want to guarantee this will be always const
    pub fn listen_addr(&self) -> SocketAddr {
        self.axum_listen_addr
    }

    /// Returns the address the discovery server is listening on.
    pub fn discovery_listen_addr(&self) -> SocketAddr {
        self.discovery.listen_addr()
    }

    /// Starts the Alpaca and discovery servers.
    ///
    /// Note: this function starts an infinite async loop and it's your responsibility to spawn it off
    /// via [`tokio::spawn`] if necessary.
    pub async fn start(self) -> eyre::Result<std::convert::Infallible> {
        match tokio::select! {
            axum = self.axum => axum?,
            discovery = self.discovery.start() => discovery,
        } {}
    }
}

#[derive(Deserialize)]
struct ApiPath {
    #[serde(with = "DevicePath")]
    device_type: DeviceType,
    device_number: usize,
    action: String,
}

impl Server {
    /// Binds the Alpaca and discovery servers to local ports.
    pub async fn bind(self) -> eyre::Result<BoundServer> {
        let addr = self.listen_addr;

        tracing::debug!(%addr, "Binding Alpaca server");

        // Like in discovery, use dual stack (IPv4+IPv6) consistently on all platforms.
        //
        // This is usually what user wants when setting IPv6 address like `[::]`
        // and this is what happens by default on popular Linux distros but not on Windows.
        //
        // For that, we can't use the standard `TcpListener::bind` and need to build our own socket.
        let socket = socket2::Socket::new(
            socket2::Domain::for_address(addr),
            socket2::Type::STREAM,
            Some(socket2::Protocol::TCP),
        )?;

        if addr.is_ipv6() {
            socket.set_only_v6(false)?;
        }

        socket.set_nonblocking(true)?;
        socket.bind(&addr.into())?;
        socket.listen(128)?;

        let listener = tokio::net::TcpListener::from_std(socket.into())?;

        // The address can differ e.g. when using port 0 (auto-assigned).
        let bound_addr = listener.local_addr()?;

        tracing::info!(%bound_addr, "Bound Alpaca server");

        // Bind discovery server only once the Alpaca server is bound successfully.
        // We need to know the bound address & the port to advertise.
        let discovery_server = DiscoveryServer::for_alpaca_server_at(bound_addr)
            .bind()
            .await?;

        tracing::debug!("Bound Alpaca discovery server");

        Ok(BoundServer {
            axum: async move {
                axum::serve(
                    listener,
                    self.into_router()
                        // .layer(TraceLayer::new_for_http())
                        .into_make_service(),
                )
                .await?;
                unreachable!("Alpaca server should never stop without an error")
            }
            .instrument(tracing::error_span!("alpaca_server_loop"))
            .boxed(),
            axum_listen_addr: bound_addr,
            discovery: discovery_server,
        })
    }

    /// Binds the Alpaca and discovery servers to local ports and starts them.
    ///
    /// This is a convenience method that is equivalent to calling [`Self::bind`] and [`BoundServer::start`].
    pub async fn start(self) -> eyre::Result<std::convert::Infallible> {
        self.bind().await?.start().await
    }

    #[allow(clippy::too_many_lines)]
    fn into_router(self) -> Router {
        let devices = Arc::new(self.devices);
        let server_info = Arc::new(self.info);

        Router::new()
            .route(
                "/management/apiversions",
                axum::routing::get(|server_handler: ServerHandler| {
                    server_handler.exec(|_params| async move { ValueResponse { value: [1_u32] } })
                }),
            )
            .route("/management/v1/configureddevices", {
                let this = Arc::clone(&devices);

                axum::routing::get(|server_handler: ServerHandler| {
                    server_handler.exec(|_params| async move {
                        ValueResponse {
                            value: this
                                .iter_all()
                                .map(|(device, number)| device.to_configured_device(number))
                                .collect::<Vec<_>>(),
                        }
                    })
                })
            })
            .route("/management/v1/description", {
                let server_info = Arc::clone(&server_info);

                axum::routing::get(move |server_handler: ServerHandler| {
                    server_handler.exec(|_params| async move {
                        ValueResponse {
                            value: Arc::clone(&server_info),
                        }
                    })
                })
            })
            .route("/setup", {
                let this = Arc::clone(&devices);
                let server_info = Arc::clone(&server_info);

                axum::routing::get(|| async move {
                    #[derive(TemplateOnce)]
                    #[template(path = "setup_template.html")]
                    struct TemplateContext {
                        server_info: Arc<ServerInfo>,
                        grouped_devices: BTreeMap<DeviceType, Vec<(usize, String)>>,
                    }

                    let mut ctx = TemplateContext {
                        server_info: Arc::clone(&server_info),
                        grouped_devices: BTreeMap::new(),
                    };

                    for (device, number) in this.iter_all() {
                        let device = device.to_configured_device(number);

                        ctx.grouped_devices
                            .entry(device.ty)
                            .or_default()
                            .push((number, device.name));
                    }

                    match ctx.render_once() {
                        Ok(html) => Ok(Html(html)),
                        Err(err) => {
                            tracing::error!(%err, "Failed to render setup page");
                            Err((StatusCode::INTERNAL_SERVER_ERROR, format!("{err:#}")))
                        }
                    }
                })
            })
            .route(
                "/api/v1/:device_type/:device_number/:action",
                axum::routing::any(
                    move |Path(ApiPath {
                              device_type,
                              device_number,
                              action,
                          }),
                          #[cfg(feature = "camera")] headers: http::HeaderMap,
                          server_handler: ServerHandler| async move {
                        #[cfg(feature = "camera")]
                        let mut action = action;

                        #[cfg(feature = "camera")]
                        if device_type == DeviceType::Camera {
                            // imagearrayvariant is soft-deprecated; we should accept it but
                            // forward to the imagearray handler instead.
                            if action == "imagearrayvariant" {
                                action.truncate("imagearray".len());
                            }

                            if matches!(server_handler.params, ActionParams::Get { .. })
                                && action == "imagearray"
                                && crate::api::ImageArray::is_accepted(&headers)
                            {
                                return server_handler
                                    .exec(|_params| async move {
                                        Ok::<_, Error>(crate::api::ImageBytesResponse(
                                            devices
                                                .get_for_server::<dyn Camera>(device_number)?
                                                .image_array()
                                                .await?,
                                        ))
                                    })
                                    .await;
                            }
                        }

                        // Setup endpoint is not an ASCOM method, so doesn't need the transaction and ASCOMResult wrapping.
                        if action == "setup" {
                            return match devices
                                .get_device_for_server(device_type, device_number)?
                                .setup()
                                .await
                            {
                                Ok(html) => Ok(Html(html).into_response()),
                                Err(err) => {
                                    Err((StatusCode::INTERNAL_SERVER_ERROR, format!("{err:#}"))
                                        .into())
                                }
                            };
                        }

                        server_handler
                            .exec(|params| {
                                devices.handle_action(device_type, device_number, &action, params)
                            })
                            .await
                    },
                ),
            )
    }
}
