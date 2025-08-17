use super::DEFAULT_DISCOVERY_PORT;
use crate::discovery::{
    bind_socket, get_active_interfaces, AlpacaPort, DISCOVERY_ADDR_V6, DISCOVERY_MSG,
};
use netdev::Interface;
use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use tokio::net::UdpSocket;

/// Alpaca discovery server configuration.
#[derive(Debug, Clone, Copy)]
pub struct Server {
    /// Address for the discovery server to listen on.
    pub listen_addr: SocketAddr,
    /// Port the Alpaca server is listening on.
    pub alpaca_port: u16,
}

#[tracing::instrument(level = "trace", skip_all, fields(intf.friendly_name = intf.friendly_name.as_ref(), intf.description = intf.description.as_ref(), ?intf.ipv4, ?intf.ipv6))]
fn join_multicast_group(socket: &UdpSocket, intf: &Interface) {
    match socket.join_multicast_v6(&DISCOVERY_ADDR_V6, intf.index) {
        Ok(()) => tracing::trace!("success"),
        Err(err) => tracing::warn!(%err),
    }
}

#[tracing::instrument(level = "debug", ret, skip(socket))]
fn join_multicast_groups(socket: &UdpSocket, listen_addr: Ipv6Addr) {
    if listen_addr.is_unspecified() {
        // If it's [::], join multicast on every available interface with IPv6 support.
        for intf in get_active_interfaces() {
            if !intf.ipv6.is_empty() {
                join_multicast_group(socket, &intf);
            }
        }
    } else {
        // If it's a specific address, find corresponding interface and join multicast on it.
        let intf = get_active_interfaces()
            .find(|intf| intf.ipv6.iter().any(|net| net.addr() == listen_addr))
            .expect("internal error: couldn't find the interface of an already bound socket");

        join_multicast_group(socket, &intf);
    }
}

impl Server {
    /// Creates a new discovery server for an already bound Alpaca server.
    ///
    /// This creates a configuration with the same IP address as the provided Alpaca server
    /// and the default discovery port (32227).
    ///
    /// You can modify the configuration before binding the server via [`Server::bind`].
    pub const fn for_alpaca_server_at(alpaca_addr: SocketAddr) -> Self {
        Self {
            listen_addr: SocketAddr::new(alpaca_addr.ip(), DEFAULT_DISCOVERY_PORT),
            alpaca_port: alpaca_addr.port(),
        }
    }

    /// Binds the discovery server to the specified address and port.
    #[tracing::instrument(level = "error")]
    pub async fn bind(self) -> eyre::Result<BoundServer> {
        let mut socket = bind_socket(self.listen_addr)?;
        if let IpAddr::V6(listen_addr) = self.listen_addr.ip() {
            // Both netdev::get_interfaces and join_multicast_group can take a long time.
            // Spawn them all off to the async runtime.
            socket = tokio::task::spawn_blocking(move || {
                join_multicast_groups(&socket, listen_addr);
                socket
            })
            .await?;
        }
        Ok(BoundServer {
            socket,
            response_msg: serde_json::to_string(&AlpacaPort {
                alpaca_port: self.alpaca_port,
            })?,
        })
    }
}

/// Alpaca discovery server bound to a local socket.
///
/// This struct is returned by [`Server::bind`].
#[derive(derive_more::Debug)]
pub struct BoundServer {
    socket: UdpSocket,
    #[debug(skip)]
    response_msg: String,
}

impl BoundServer {
    /// Get listen address of the discovery server.
    pub fn listen_addr(&self) -> SocketAddr {
        self.socket
            .local_addr()
            .expect("bound socket must return its address")
    }

    /// Starts a discovery server on the local network.
    ///
    /// Note: this function starts an infinite async loop and it's your responsibility
    /// to spawn it off via [`tokio::spawn`] if necessary.
    ///
    /// The return type is intentionally split off into a Result for the bound socket
    /// and a Future for the server itself. This allows consumers to ensure that the server
    /// is bound successfully before starting the infinite loop.
    #[tracing::instrument(name = "alpaca_discovery_server_loop", level = "error")]
    pub async fn start(self) -> std::convert::Infallible {
        let mut buf = [0; DISCOVERY_MSG.len() + 1];
        loop {
            if let Err(err) = async {
                let (len, src) = self.socket.recv_from(&mut buf).await?;
                let data = &buf[..len];
                if data == DISCOVERY_MSG {
                    tracing::trace!(%src, "Received Alpaca discovery request");
                    // UDP packets are sent as whole messages, no need to check length.
                    let _ = self
                        .socket
                        .send_to(self.response_msg.as_bytes(), src)
                        .await?;
                } else {
                    tracing::warn!(%src, "Received unknown packet");
                }
                Ok::<_, std::io::Error>(())
            }
            .await
            {
                tracing::error!(%err, "Error while handling a discovery request");
            }
        }
    }
}
