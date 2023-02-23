use net_literals::{addr, ipv6};
use std::net::Ipv6Addr;

const MULTICAST_ADDR_V6: Ipv6Addr = ipv6!("ff12::a1:9aca");
const DISCOVERY_MSG: &[u8] = b"alpacadiscovery1";

#[tracing::instrument(err)]
pub async fn start_server(alpaca_port: u16) -> anyhow::Result<()> {
    tracing::debug!("Starting Alpaca discovery server");
    let response_msg = format!(r#"{{"AlpacaPort":{alpaca_port}}}"#);
    let socket = tokio::net::UdpSocket::bind(addr!("[::]:32227")).await?;
    socket.join_multicast_v6(&MULTICAST_ADDR_V6, 0)?;
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
