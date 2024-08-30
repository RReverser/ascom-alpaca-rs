use crate::api::TypedDevice;
use crate::discovery::{
    bind_socket, get_active_interfaces, AlpacaPort, DEFAULT_DISCOVERY_PORT, DISCOVERY_ADDR_V6,
    DISCOVERY_MSG,
};
use futures::StreamExt;
use netdev::interface::InterfaceType;
use netdev::Interface;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Duration;
use tokio::net::UdpSocket;
use tracing_futures::Instrument;

/// Discovery client.
#[derive(Debug, Clone, Copy)]
pub struct Client {
    /// Number of discovery requests to send.
    ///
    /// Defaults to 2.
    pub num_requests: usize,
    /// Time to wait after each discovered device for more responses.
    ///
    /// Defaults to 1 second.
    pub timeout: Duration,
    /// Discovery port to send requests to.
    ///
    /// Defaults to 32227.
    pub discovery_port: u16,
}

/// Bound discovery client ready to send discovery requests.
///
/// This can be obtained by calling [`Client::bind`] and stored for reuse.
#[derive(Debug)]
pub struct BoundClient {
    client: Client,
    socket: UdpSocket,
    interfaces: Vec<Interface>,
    buf: Vec<u8>,
    seen: Vec<SocketAddr>,
}

impl BoundClient {
    #[tracing::instrument(level = "trace", skip_all, fields(%addr, intf.friendly_name = intf.friendly_name.as_ref(), intf.description = intf.description.as_ref(), ?intf.ipv4, ?intf.ipv6))]
    async fn send_discovery_msg(&self, addr: Ipv6Addr, intf: &Interface) {
        let send_op = async {
            if addr.is_multicast() {
                socket2::SockRef::from(&self.socket).set_multicast_if_v6(intf.index)?;
            }
            // UDP packets are sent as whole messages, no need to check length.
            let _ = self
                .socket
                .send_to(DISCOVERY_MSG, (addr, self.client.discovery_port))
                .await?;
            Ok::<_, std::io::Error>(())
        };
        match send_op.await {
            Ok(()) => tracing::trace!("success"),
            Err(err) => tracing::warn!(%err),
        }
    }

    #[tracing::instrument(level = "debug", skip_all)]
    async fn send_discovery_msgs(&self) {
        for intf in &self.interfaces {
            for net in &intf.ipv4 {
                let broadcast = Ipv4Addr::from(u32::from(net.addr) | !u32::from(net.netmask));

                self.send_discovery_msg(broadcast.to_ipv6_mapped(), intf)
                    .await;
            }

            if !intf.ipv6.is_empty() {
                self.send_discovery_msg(
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
    }

    #[tracing::instrument(level = "debug", ret, err(level = "warn"), skip_all)]
    async fn recv_discovery_response(&mut self) -> eyre::Result<SocketAddr> {
        self.buf.clear();
        let (len, addr) = self.socket.recv_buf_from(&mut self.buf).await?;
        let AlpacaPort { alpaca_port } = serde_json::from_slice(&self.buf[..len])?;
        let ip = match addr.ip() {
            IpAddr::V6(ip) => ip,
            IpAddr::V4(_) => unreachable!(
                "shouldn't be able to get response from unmapped IPv4 address on IPv6 socket"
            ),
        };
        // We used IPv6 socket to send IPv4 requests as well by using mapped addresses;
        // now that we got responses, we need to remap them back to IPv4.
        let ip = ip.to_ipv4_mapped().map_or(IpAddr::V6(ip), IpAddr::V4);
        Ok(SocketAddr::new(ip, alpaca_port))
    }

    /// Discover Alpaca servers on the local network.
    ///
    /// This function returns a stream of discovered device addresses.
    pub fn discover_addrs(&mut self) -> impl '_ + futures::Stream<Item = SocketAddr> {
        async_fn_stream::fn_stream(|emitter| async move {
            self.seen.clear();

            for _ in 0..self.client.num_requests {
                self.send_discovery_msgs().await;

                while let Ok(result) =
                    tokio::time::timeout(self.client.timeout, self.recv_discovery_response()).await
                {
                    match result {
                        Ok(addr) if !self.seen.contains(&addr) => {
                            self.seen.push(addr);
                            emitter.emit(addr).await;
                        }
                        _ => {}
                    }
                }
            }
        })
        .instrument(tracing::error_span!("discover_addrs"))
    }

    /// Discover all devices on the local network.
    ///
    /// This function returns a stream of discovered devices.
    ///
    /// Note that it might return duplicates if same server is accessible on multiple network interfaces.
    /// You can collect devices into [`Devices`](crate::Devices), [`HashSet`](std::collections::HashSet) or a similar data structure to deduplicate them.
    ///
    /// This function will log but otherwise ignore errors from discovered but unreachable servers.
    /// If you need more control, use [`Self::discover_addrs`] and [`crate::Client::new_from_addr`] directly instead.
    pub fn discover_devices(&mut self) -> impl '_ + futures::Stream<Item = TypedDevice> {
        self.discover_addrs()
            .filter_map(|addr| async move {
                match crate::Client::new_from_addr(addr).get_devices().await {
                    Ok(devices) => Some(devices),
                    Err(err) => {
                        tracing::warn!(%addr, %err, "failed to retrieve list of devices");
                        None
                    }
                }
            })
            .flat_map_unordered(None, futures::stream::iter)
            .instrument(tracing::error_span!("discover_devices"))
    }
}

impl Client {
    /// Create a discovery client with default settings.
    pub const fn new() -> Self {
        Self {
            num_requests: 2,
            timeout: Duration::from_secs(1),
            discovery_port: DEFAULT_DISCOVERY_PORT,
        }
    }

    /// Bind the client to a local address.
    #[tracing::instrument(level = "error")]
    pub async fn bind(self) -> eyre::Result<BoundClient> {
        let socket = bind_socket((Ipv6Addr::UNSPECIFIED, 0)).await?;
        let interfaces = tokio::task::spawn_blocking(|| get_active_interfaces().collect()).await?;
        Ok(BoundClient {
            client: self,
            socket,
            interfaces,
            buf: Vec::with_capacity(64),
            seen: Vec::new(),
        })
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}
