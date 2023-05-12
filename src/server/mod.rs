mod discovery;
pub use discovery::Server as DiscoveryServer;

mod transaction;
pub(crate) use transaction::*;

mod case_insensitive_str;

mod params;
pub(crate) use params::ActionParams;

mod response;
pub(crate) use response::Response;

mod error;
pub(crate) use error::{Error, Result};

#[cfg(feature = "camera")]
use crate::api::{Camera, DeviceType};
use crate::api::{CargoServerInfo, DevicePath, ServerInfo};
use crate::discovery::DEFAULT_DISCOVERY_PORT;
use crate::response::ValueResponse;
use crate::Devices;
use axum::body::HttpBody;
use axum::extract::{FromRequest, Path};
use axum::http::Request;
use axum::response::IntoResponse;
use axum::routing::MethodFilter;
use axum::{BoxError, Router};
use net_literals::addr;
use std::future::Future;
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

struct ServerHandler {
    path: String,
    params: ActionParams,
}

#[async_trait::async_trait]
impl<S, B> FromRequest<S, B> for ServerHandler
where
    B: HttpBody + Send + Sync + 'static,
    B::Data: Send,
    B::Error: Into<BoxError>,
    S: Send + Sync,
{
    type Rejection = axum::response::Response;

    async fn from_request(
        req: Request<B>,
        state: &S,
    ) -> std::result::Result<Self, Self::Rejection> {
        let path = req.uri().path().to_owned();
        let params = ActionParams::from_request(req, state).await?;
        Ok(Self { path, params })
    }
}

impl ServerHandler {
    async fn exec<Resp: Response, RespFut: Future<Output = Resp> + Send>(
        mut self,
        make_response: impl FnOnce(ActionParams) -> RespFut + Send,
    ) -> axum::response::Response {
        let request_transaction = match RequestTransaction::extract(&mut self.params) {
            Ok(transaction) => transaction,
            Err(err) => {
                return (axum::http::StatusCode::BAD_REQUEST, format!("{err:#}")).into_response();
            }
        };
        let response_transaction =
            ResponseTransaction::new(request_transaction.client_transaction_id);

        let span = tracing::error_span!(
            "handle_alpaca_request",
            path = self.path,
            client_id = request_transaction.client_id,
            client_transaction_id = request_transaction.client_transaction_id,
            server_transaction_id = response_transaction.server_transaction_id,
        );

        async move {
            tracing::debug!(params = ?self.params, "Received request");

            make_response(self.params)
                .await
                .into_axum(response_transaction)
        }
        .instrument(span)
        .await
    }
}

impl Server {
    /// Starts the Alpaca and discovery servers.
    ///
    /// The discovery server will be started only after the Alpaca server is bound to a port successfully.
    ///
    /// Note: this function starts an infinite async loop and it's your responsibility to spawn it off
    /// via [`tokio::spawn`] if necessary.
    ///
    /// The return type is intentionally split into an outer Result that indicates whether the sockets were
    /// bound successfully and an inner Future that is actually running the infinite request loops.
    pub async fn start(
        self,
    ) -> eyre::Result<impl Future<Output = eyre::Result<std::convert::Infallible>>> {
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

        socket.bind(&addr.into())?;

        let server = axum::Server::from_tcp(socket.into())?.serve(
            self.into_router()
                // .layer(TraceLayer::new_for_http())
                .into_make_service(),
        );

        // The address can differ e.g. when using port 0 (auto-assigned).
        let bound_addr = server.local_addr();

        tracing::info!(%bound_addr, "Bound Alpaca server");

        // Bind discovery server only once the Alpaca server is bound successfully.
        // We need to know the bound address & the port to advertise.
        let discovery_server = DiscoveryServer::for_alpaca_server_at(bound_addr)
            .bind()
            .await?;

        tracing::debug!("Bound Alpaca discovery server");

        Ok(async move {
            tokio::select! {
                server_result = server.instrument(tracing::error_span!("alpaca_server_loop")) => {
                    server_result?;
                    unreachable!("Alpaca server should never stop without an error");
                }
                never_returns = discovery_server.start() => match never_returns {},
            }
        })
    }

    fn into_router(self) -> Router {
        let devices = Arc::new(self.devices);
        let server_info = Arc::new(self.info);

        Router::new()
            .route(
                "/management/apiversions",
                axum::routing::get(|server_handler: ServerHandler| {
                    server_handler.exec(|_params| async move { ValueResponse::from([1_u32]) })
                }),
            )
            .route("/management/v1/configureddevices", {
                let this = Arc::clone(&devices);

                axum::routing::get(|server_handler: ServerHandler| {
                    server_handler.exec(|_params| async move {
                        let devices = this
                            .iter_all()
                            .map(|(device, number)| device.to_configured_device(number))
                            .collect::<Vec<_>>();
                        ValueResponse::from(devices)
                    })
                })
            })
            .route(
                "/management/v1/description",
                axum::routing::get(move |server_handler: ServerHandler| {
                    server_handler.exec(|_params| async move {
                        ValueResponse::from(Arc::clone(&server_info))
                    })
                }),
            )
            .route(
                "/api/v1/:device_type/:device_number/:action",
                axum::routing::on(
                    MethodFilter::GET | MethodFilter::PUT,
                    move |Path((DevicePath(device_type), device_number, action)): Path<(
                        DevicePath,
                        usize,
                        String,
                    )>,
                          #[cfg(feature = "camera")] headers: axum::http::HeaderMap,
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
