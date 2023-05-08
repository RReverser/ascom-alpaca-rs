use crate::discovery::{
    bind_socket, AlpacaPort, DEFAULT_DISCOVERY_PORT, DISCOVERY_ADDR_V6, DISCOVERY_MSG,
};
use default_net::interface::InterfaceType;
use default_net::Interface;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Duration;
use tokio::net::UdpSocket;

/// Discovery client.
#[derive(Debug, Clone, Copy)]
pub struct Client {
    /// Number of discovery requests to send.
    ///
    /// Defaults to 1.
    pub num_requests: usize,
    /// Time to wait after each discovered device for more responses.
    ///
    /// Defaults to 1 seconds.
    pub timeout: Duration,
    /// Discovery port to send requests to.
    ///
    /// Defaults to 32227.
    pub discovery_port: u16,
}

impl Client {
    /// Create a discovery client with default settings.
    pub const fn new() -> Self {
        Self {
            num_requests: 1,
            timeout: Duration::from_secs(3),
            discovery_port: DEFAULT_DISCOVERY_PORT,
        }
    }

    #[tracing::instrument(ret, err, skip_all, fields(%addr, intf.friendly_name = intf.friendly_name.as_ref(), intf.description = intf.description.as_ref(), ?intf.ipv4, ?intf.ipv6), level = "debug")]
    async fn send_discovery_msg(
        &self,
        ipv6_socket: &UdpSocket,
        addr: Ipv6Addr,
        intf: &Interface,
    ) -> eyre::Result<()> {
        if addr.is_multicast() {
            socket2::SockRef::from(ipv6_socket).set_multicast_if_v6(intf.index)?;
        }
        let _ = ipv6_socket
            .send_to(DISCOVERY_MSG, (addr, self.discovery_port))
            .await?;
        Ok(())
    }

    #[tracing::instrument(ret, err, skip_all, level = "debug")]
    async fn recv_discovery_response(
        &self,
        socket: &UdpSocket,
        buf: &mut [u8],
    ) -> eyre::Result<Option<SocketAddr>> {
        let (len, addr) = match tokio::time::timeout(self.timeout, socket.recv_from(buf)).await {
            Ok(result) => result?,
            Err(_timeout) => return Ok(None),
        };
        let AlpacaPort { alpaca_port } = serde_json::from_slice(&buf[..len])?;
        let ip = match addr.ip() {
            IpAddr::V6(ip) => ip,
            IpAddr::V4(_) => unreachable!(
                "shouldn't be able to get response from unmapped IPv4 address on IPv6 socket"
            ),
        };
        // We used IPv6 socket to send IPv4 requests as well by using mapped addresses;
        // now that we got responses, we need to remap them back to IPv4.
        let ip = ip.to_ipv4_mapped().map_or(IpAddr::V6(ip), IpAddr::V4);
        Ok(Some(SocketAddr::new(ip, alpaca_port)))
    }

    /// Discover Alpaca servers on the local network.
    ///
    /// This function returns a stream of discovered device addresses.
    /// `each_timeout` determines how long to wait after each discovered device.
    #[tracing::instrument(err, level = "debug")]
    #[allow(clippy::panic_in_result_fn)] // unreachable! is fine here
    pub async fn discover_addrs(self) -> eyre::Result<impl futures::Stream<Item = SocketAddr>> {
        let interfaces = tokio::task::spawn_blocking(default_net::get_interfaces).await?;
        let v6_socket = bind_socket((Ipv6Addr::UNSPECIFIED, 0)).await?;
        Ok(async_stream::stream!({
            let mut seen = Vec::new();
            let mut buf = [0; 64];

            for _ in 0..self.num_requests {
                for intf in &interfaces {
                    for net in &intf.ipv4 {
                        let broadcast =
                            Ipv4Addr::from(u32::from(net.addr) | !u32::from(net.netmask));

                        let _ = self
                            .send_discovery_msg(&v6_socket, broadcast.to_ipv6_mapped(), intf)
                            .await;
                    }

                    if !intf.ipv6.is_empty() {
                        let _ = self
                            .send_discovery_msg(
                                &v6_socket,
                                if intf.if_type == InterfaceType::Loopback {
                                    // Loopback interface doesn't have a link-local address
                                    // so it can't be used for multicast.
                                    Ipv6Addr::LOCALHOST
                                } else {
                                    DISCOVERY_ADDR_V6
                                },
                                intf,
                            )
                            .await;
                    }
                }

                while let Some(result) = self
                    .recv_discovery_response(&v6_socket, &mut buf)
                    .await
                    .transpose()
                {
                    if let Ok(addr) = result {
                        if !seen.contains(&addr) {
                            seen.push(addr);
                            yield addr;
                        }
                    }
                }
            }
        }))
    }
}
