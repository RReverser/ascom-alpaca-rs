use net_literals::ipv6;
use serde::{Deserialize, Serialize};
use std::net::{Ipv6Addr, SocketAddr};

pub(crate) const DISCOVERY_ADDR_V6: Ipv6Addr = ipv6!("ff12::a1:9aca");
pub(crate) const DISCOVERY_MSG: &[u8] = b"alpacadiscovery1";
pub(crate) const DEFAULT_DISCOVERY_PORT: u16 = 32227;

#[derive(Serialize, Deserialize)]
pub(crate) struct AlpacaPort {
    #[serde(rename = "AlpacaPort")]
    pub(crate) alpaca_port: u16,
}

pub(crate) fn bind_socket(port: u16) -> anyhow::Result<tokio::net::UdpSocket> {
    let socket = socket2::Socket::new(
        socket2::Domain::IPV6,
        socket2::Type::DGRAM,
        Some(socket2::Protocol::UDP),
    )?;
    socket.set_nonblocking(true)?;
    socket.set_only_v6(false)?;
    socket.bind(&socket2::SockAddr::from(SocketAddr::from((
        Ipv6Addr::UNSPECIFIED,
        port,
    ))))?;
    Ok(tokio::net::UdpSocket::from_std(socket.into())?)
}

pub use crate::client::DiscoveryClient;
pub use crate::server::DiscoveryServer;

#[cfg(test)]
#[tokio::test]
async fn test_discovery() -> anyhow::Result<()> {
    use futures::TryStreamExt;
    use std::net::SocketAddr;

    tokio::select!(
        result = DiscoveryServer::new(8378).start_server() => result,
        addrs = DiscoveryClient::new().discover_addrs().try_collect::<Vec<_>>() => {
            let addrs = addrs?;
            anyhow::ensure!(
                matches!(
                    addrs.as_slice(),
                    [SocketAddr::V4(addr)] if addr.port() == 8378
                ),
                "Couldn't find own discovery server on port 8378. Found: {addrs:?}"
            );
            Ok(())
        }
    )
}
