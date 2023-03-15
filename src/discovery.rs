use net_literals::{addr, ipv6};
use serde::{Deserialize, Serialize};
use std::net::{Ipv6Addr, SocketAddr};
use std::time::Duration;

const DISCOVERY_ADDR: Ipv6Addr = ipv6!("ff12::a1:9aca");
const DISCOVERY_MSG: &[u8] = b"alpacadiscovery1";
const DISCOVERY_PORT: u16 = 32227;

#[derive(Serialize, Deserialize)]
struct AlpacaPort {
    #[serde(rename = "AlpacaPort")]
    alpaca_port: u16,
}

#[tracing::instrument(err)]
pub async fn start_server(alpaca_port: u16) -> anyhow::Result<()> {
    tracing::debug!("Starting Alpaca discovery server");
    let response_msg = serde_json::to_string(&AlpacaPort { alpaca_port })?;
    let socket = tokio::net::UdpSocket::bind((Ipv6Addr::UNSPECIFIED, DISCOVERY_PORT)).await?;
    socket.join_multicast_v6(&DISCOVERY_ADDR, 0)?;
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

#[tracing::instrument]
pub fn discover(each_timeout: Duration) -> impl futures::Stream<Item = anyhow::Result<SocketAddr>> {
    async_stream::try_stream! {
        tracing::debug!("Starting Alpaca discovery");
        let socket = tokio::net::UdpSocket::bind(addr!("[::]:0")).await?;
        tracing::debug!("Sending discovery request");
        let _ = socket
            .send_to(DISCOVERY_MSG, (DISCOVERY_ADDR, DISCOVERY_PORT))
            .await?;
        let mut buf = [0; 32]; // "{"AlpacaPort":12345}" + some extra bytes for spaces just in case
        loop {
            let (len, src) =
                match tokio::time::timeout(each_timeout, socket.recv_from(&mut buf)).await {
                    Ok(result) => result?,
                    Err(_timeout) => {
                        tracing::debug!("Ending discovery");
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
