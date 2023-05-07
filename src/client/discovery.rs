use crate::discovery::{
    bind_socket, AlpacaPort, DEFAULT_DISCOVERY_PORT, DISCOVERY_ADDR_V6, DISCOVERY_MSG,
};
use default_net::interface::InterfaceType;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Duration;
use tokio::net::UdpSocket;
use tracing::Instrument;

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

    #[tracing::instrument(err, skip(self))]
    async fn send_discovery_msg(
        &self,
        ipv6_socket: &UdpSocket,
        addr: impl Into<IpAddr> + std::fmt::Debug + Send,
    ) -> eyre::Result<()> {
        let addr = addr.into();
        tracing::debug!("Sending Alpaca discovery request");
        let _ = ipv6_socket
            .send_to(DISCOVERY_MSG, (addr, self.discovery_port))
            .await?;
        Ok(())
    }

    /// Discover Alpaca servers on the local network.
    ///
    /// This function returns a stream of discovered device addresses.
    /// `each_timeout` determines how long to wait after each discovered device.
    #[tracing::instrument]
    #[allow(clippy::panic_in_result_fn)] // unreachable! is fine here
    pub fn discover_addrs(self) -> eyre::Result<impl futures::Stream<Item = SocketAddr>> {
        tracing::debug!("Starting Alpaca discovery");
        let interfaces = default_net::get_interfaces();
        let v6_socket = bind_socket((Ipv6Addr::UNSPECIFIED, 0))?;
        Ok(async_stream::stream!({
            let v4_broadcast_dests = interfaces
                .iter()
                .flat_map(|intf| intf.ipv4.iter())
                .filter(|net| net.addr.is_loopback() || net.addr.is_private())
                .map(|net| {
                    let addr = u32::from(net.addr);
                    let mask = u32::from(net.netmask);
                    let broadcast = addr | !mask;
                    Ipv4Addr::from(broadcast)
                })
                .collect::<Vec<_>>();
            let v6_network_interfaces = interfaces
                .iter()
                .filter(|net| net.if_type != InterfaceType::Loopback)
                .collect::<Vec<_>>();
            let mut seen = Vec::new();
            let mut buf = [0; 64];
            for _ in 0..self.num_requests {
                for &dest in &v4_broadcast_dests {
                    let _ = self
                        .send_discovery_msg(&v6_socket, dest.to_ipv6_mapped())
                        .await;
                }
                // we had to exclude IPv6 loopback interface as it doesn't support multicast;
                // send a separate unicast message just to loopback in case there's a server
                // that's listening only on ::1
                let _ = self
                    .send_discovery_msg(&v6_socket, Ipv6Addr::LOCALHOST)
                    .await;
                for &intf in &v6_network_interfaces {
                    if socket2::SockRef::from(&v6_socket)
                        .set_multicast_if_v6(intf.index)
                        .is_ok()
                    {
                        let _ = self
                            .send_discovery_msg(&v6_socket, DISCOVERY_ADDR_V6)
                            .instrument(tracing::debug_span!("Multicasting over", ?intf))
                            .await;
                    }
                }
                while let Ok(result) =
                    tokio::time::timeout(self.timeout, v6_socket.recv_from(&mut buf)).await
                {
                    match async {
                        let (len, addr) = result?;
                        let AlpacaPort { alpaca_port } = serde_json::from_slice(&buf[..len])?;
                        let ip = match addr.ip() {
                            IpAddr::V6(ip) => ip,
                            IpAddr::V4(_) => unreachable!("shouldn't be able to get response from unmapped IPv4 address on IPv6 socket"),
                        };
                        // We used IPv6 socket to send IPv4 requests as well by using mapped addresses;
                        // now that we got responses, we need to remap them back to IPv4.
                        let ip = ip.to_ipv4_mapped().map_or(IpAddr::V6(ip), IpAddr::V4);
                        let addr = SocketAddr::new(ip, alpaca_port);
                        Ok::<_, eyre::Error>(if seen.contains(&addr) {
                            None
                        } else {
                            seen.push(addr);
                            tracing::debug!(?addr, "Discovered new Alpaca device");
                            Some(addr)
                        })
                    }.await {
                        Ok(Some(addr)) => yield addr,
                        Ok(None) => {}
                        Err(err) => tracing::warn!(?err, "Error while parsing discovery response"),
                    }
                }
            }
        }))
    }
}
