use super::DEFAULT_DISCOVERY_PORT;
use crate::discovery::{bind_socket, AlpacaPort, DISCOVERY_ADDR_V6, DISCOVERY_MSG};
use default_net::Interface;
use eyre::ContextCompat;
use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use tokio::net::UdpSocket;

/// Alpaca discovery server.
#[derive(Debug, Clone, Copy)]
pub struct Server {
    /// Port of the running server.
    pub alpaca_port: u16,
    /// Discovery address to listen on.
    ///
    /// Defaults to `/* Alpaca server address */:32227`.
    pub listen_addr: SocketAddr,
}

#[tracing::instrument(ret, err, skip_all, fields(intf.friendly_name = intf.friendly_name.as_ref(), intf.description = intf.description.as_ref(), ?intf.ipv4, ?intf.ipv6), level = "debug")]
fn join_multicast_group(socket: &UdpSocket, intf: &Interface) -> eyre::Result<()> {
    socket.join_multicast_v6(&DISCOVERY_ADDR_V6, intf.index)?;
    Ok(())
}

#[allow(clippy::panic_in_result_fn)] // false positive, triggers inside `tracing::instrument` macro
#[tracing::instrument(skip(socket))]
fn join_multicast_groups(socket: UdpSocket, listen_addr: Ipv6Addr) -> eyre::Result<UdpSocket> {
    let interfaces = default_net::get_interfaces();
    if listen_addr.is_unspecified() {
        // If it's [::], join multicast on every available interface with IPv6 support.
        for intf in interfaces {
            if !intf.ipv6.is_empty() {
                let _ = join_multicast_group(&socket, &intf);
            }
        }
    } else {
        // If it's a specific address, find corresponding interface and join multicast on it.
        let intf = interfaces
            .iter()
            .find(|intf| intf.ipv6.iter().any(|net| net.addr == listen_addr))
            .with_context(|| format!("No interface found for {listen_addr}"))?;

        let _ = join_multicast_group(&socket, intf);
    }
    Ok(socket)
}

impl Server {
    /// Creates a new discovery server for Alpaca server running at the specified address.
    pub const fn for_alpaca_server_at(alpaca_listen_addr: SocketAddr) -> Self {
        Self {
            alpaca_port: alpaca_listen_addr.port(),
            listen_addr: SocketAddr::new(alpaca_listen_addr.ip(), DEFAULT_DISCOVERY_PORT),
        }
    }

    /// Starts a discovery server on the local network.
    ///
    /// Note: this function starts an infinite async loop and it's your responsibility
    /// to spawn it off via [`tokio::spawn`] if necessary.
    ///
    /// The return type is intentionally split off into a Result for the bound socket
    /// and a Future for the server itself. This allows consumers to ensure that the server
    /// is bound successfully before starting the infinite loop.
    #[tracing::instrument(err, level = "debug")]
    pub async fn start(
        self,
    ) -> eyre::Result<impl futures::Future<Output = std::convert::Infallible>> {
        let response_msg = serde_json::to_string(&AlpacaPort {
            alpaca_port: self.alpaca_port,
        })?;
        let socket = bind_socket(self.listen_addr).await?;
        let socket = if let IpAddr::V6(listen_addr) = self.listen_addr.ip() {
            // Both default_net::get_interfaces and join_multicast_group can take a long time.
            // Spawn them all off to the async runtime.
            tokio::task::spawn_blocking(move || join_multicast_groups(socket, listen_addr))
                .await??
        } else {
            socket
        };
        Ok(async move {
            let mut buf = [0; DISCOVERY_MSG.len() + 1];
            loop {
                if let Err(err) = async {
                    let (len, src) = socket.recv_from(&mut buf).await?;
                    let data = &buf[..len];
                    if data == DISCOVERY_MSG {
                        tracing::debug!(%src, "Received Alpaca discovery request");
                        eyre::ensure!(
                            socket.send_to(response_msg.as_bytes(), src).await?
                                == response_msg.len(),
                            "Failed to send discovery response",
                        );
                    } else {
                        tracing::warn!(%src, "Received unknown multicast packet");
                    }
                    Ok(())
                }
                .await
                {
                    tracing::error!(%err, "Error while handling a discovery request");
                }
            }
        })
    }
}
