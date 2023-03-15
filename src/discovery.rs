use net_literals::{addr, ipv6};
use serde::{Deserialize, Serialize};
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Duration;

const DISCOVERY_ADDR_V6: Ipv6Addr = ipv6!("ff12::a1:9aca");
const DISCOVERY_MSG: &[u8] = b"alpacadiscovery1";
const DEFAULT_DISCOVERY_PORT: u16 = 32227;

#[derive(Serialize, Deserialize)]
struct AlpacaPort {
    #[serde(rename = "AlpacaPort")]
    alpaca_port: u16,
}

#[derive(Debug, Clone, Copy)]
pub struct DiscoveryServer {
    /// Port of the running server.
    pub alpaca_port: u16,
    /// Discovery port to listen on.
    ///
    /// Defaults to 32227.
    pub discovery_port: u16,
}

impl DiscoveryServer {
    /// Creates a new discovery server for Alpaca server running at specified port.
    pub const fn new(alpaca_port: u16) -> Self {
        Self {
            alpaca_port,
            discovery_port: DEFAULT_DISCOVERY_PORT,
        }
    }

    /// Starts a discovery server on the local network.
    #[tracing::instrument(err)]
    pub async fn start_server(self) -> anyhow::Result<()> {
        tracing::debug!("Starting Alpaca discovery server");
        let response_msg = serde_json::to_string(&AlpacaPort {
            alpaca_port: self.alpaca_port,
        })?;
        let socket =
            tokio::net::UdpSocket::bind((Ipv6Addr::UNSPECIFIED, self.discovery_port)).await?;
        socket.join_multicast_v6(&DISCOVERY_ADDR_V6, 0)?;
        let mut buf = [0; 16];
        loop {
            let (len, src) = socket.recv_from(&mut buf).await?;
            let data = &buf[..len];
            if data == DISCOVERY_MSG {
                tracing::debug!(%src, "Received Alpaca discovery request");
                anyhow::ensure!(
                    socket.send_to(response_msg.as_bytes(), src).await? == response_msg.len(),
                    "Failed to send discovery response"
                );
            } else {
                tracing::warn!(%src, "Received unknown multicast packet");
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DiscoveryClient {
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

impl DiscoveryClient {
    /// Create a discovery client with default settings.
    pub const fn new() -> Self {
        Self {
            num_requests: 1,
            timeout: Duration::from_secs(1),
            include_ipv6: false,
            discovery_port: DEFAULT_DISCOVERY_PORT,
        }
    }

    /// Discover Alpaca devices on the local network.
    ///
    /// This function returns a stream of discovered device addresses.
    /// `each_timeout` determines how long to wait after each discovered device.
    #[tracing::instrument]
    pub fn discover(self) -> impl futures::Stream<Item = anyhow::Result<SocketAddr>> {
        async_stream::try_stream! {
            tracing::debug!("Starting Alpaca discovery");
            let socket = tokio::net::UdpSocket::bind(addr!("[::]:0")).await?;
            socket.set_broadcast(true)?;
            for request_num in 0..self.num_requests {
                if self.include_ipv6 {
                    tracing::debug!(request_num, "Sending IPv6 discovery request");
                    let _ = socket
                        .send_to(DISCOVERY_MSG, (DISCOVERY_ADDR_V6, self.discovery_port))
                        .await?;
                }
                tracing::debug!(request_num, "Sending IPv4 discovery request");
                let _ = socket.send_to(DISCOVERY_MSG, (Ipv4Addr::BROADCAST, self.discovery_port)).await?;
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
                            let addr = SocketAddr::new(src.ip(), alpaca_port);
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
