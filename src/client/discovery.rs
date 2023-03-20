use crate::discovery::{
    bind_socket, AlpacaPort, DEFAULT_DISCOVERY_PORT, DISCOVERY_ADDR_V6, DISCOVERY_MSG,
};
use net_literals::ipv6;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

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
    /// Whether to send IPv6 discovery requests.
    ///
    /// Disabled by default as all Alpaca devices should support IPv4.
    pub include_ipv6: bool,
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
            timeout: Duration::from_secs(1),
            include_ipv6: false,
            discovery_port: DEFAULT_DISCOVERY_PORT,
        }
    }

    /// Discover Alpaca servers on the local network.
    ///
    /// This function returns a stream of discovered device addresses.
    /// `each_timeout` determines how long to wait after each discovered device.
    #[tracing::instrument]
    pub fn discover_addrs(self) -> impl futures::Stream<Item = anyhow::Result<SocketAddr>> {
        async_stream::try_stream! {
            tracing::debug!("Starting Alpaca discovery");
            let socket = bind_socket(0)?;
            socket.set_broadcast(true)?;
            for request_num in 0..self.num_requests {
                if self.include_ipv6 {
                    tracing::debug!(request_num, "Sending IPv6 discovery request");
                    let _ = socket
                        .send_to(DISCOVERY_MSG, (DISCOVERY_ADDR_V6, self.discovery_port))
                        .await?;
                }
                tracing::debug!(request_num, "Sending IPv4 discovery request");
                let _ = socket.send_to(DISCOVERY_MSG, (ipv6!("::ffff:255.255.255.255"), self.discovery_port)).await?;
                let mut buf = [0; 32]; // "{"AlpacaPort":12345}" + some extra bytes for spaces just in case
                loop {
                    let (len, src) =
                        match tokio::time::timeout(self.timeout, socket.recv_from(&mut buf)).await {
                            Ok(result) => result?,
                            Err(_timeout) => {
                                tracing::debug!("Ending discovery due to timeout");
                                break;
                            }
                        };
                    let data = &buf[..len];
                    match serde_json::from_slice::<AlpacaPort>(data) {
                        Ok(AlpacaPort { alpaca_port }) => {
                            let mut ip = src.ip();
                            // TODO: use `SocketAddr::to_canonical` when it's stable.
                            if let IpAddr::V6(ip_v6) = ip {
                                if let Some(ip_v4) = ip_v6.to_ipv4_mapped() {
                                    ip = IpAddr::V4(ip_v4);
                                }
                            }
                            let addr = SocketAddr::new(ip, alpaca_port);
                            tracing::debug!(%addr, "Received Alpaca discovery response");
                            yield addr;
                        }
                        Err(err) => {
                            tracing::warn!(%src, %err, "Received unknown multicast packet");
                        }
                    }
                }
            }
        }
    }
}
