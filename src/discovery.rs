//! Discovery of Alpaca devices on the local network.

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
    // For async code, we need to set the socket to non-blocking mode.
    socket.set_nonblocking(true)?;
    // We want to talk to the IPv4 broadcast address from the same socket.
    // Using `socket2` seems to be the only way to do this from safe Rust.
    socket.set_only_v6(false)?;
    socket.bind(&socket2::SockAddr::from(SocketAddr::from((
        Ipv6Addr::UNSPECIFIED,
        port,
    ))))?;
    Ok(tokio::net::UdpSocket::from_std(socket.into())?)
}

#[cfg(feature = "client")]
pub use crate::client::DiscoveryClient;
#[cfg(feature = "server")]
pub use crate::server::DiscoveryServer;

#[cfg(test)]
#[tokio::test]
async fn test_discovery() -> anyhow::Result<()> {
    use futures::TryStreamExt;
    use net_literals::addr;

    async fn run_test(
        addr: SocketAddr,
        include_ipv6: bool,
        on_found: impl FnOnce(&mut [SocketAddr]) -> bool + Copy + Send,
    ) -> anyhow::Result<()> {
        // check that `include_ipv6` makes no difference for IPv4 addresses, just in case.
        let include_ipv6_values = if addr.is_ipv4() {
            &[false, true]
        } else {
            std::slice::from_ref(&include_ipv6)
        };

        for &include_ipv6 in include_ipv6_values {
            let mut client = DiscoveryClient::new();
            client.include_ipv6 = include_ipv6;

    tokio::select!(
                result = DiscoveryServer::for_alpaca_server_at(addr).start() => result,
                addrs = client.discover_addrs().try_collect::<Vec<_>>() => {
                    let mut addrs = addrs?;
            anyhow::ensure!(
                        on_found(&mut addrs),
                        "Couldn't find own discovery server {addr:?}. Found: {addrs:?}"
            );
            Ok(())
        }
            )?;
        }

        Ok(())
    }

    let loopback_v4 = addr!("127.0.0.1:8378");
    let loopback_v6 = addr!("[::1]:8378");
    let unspecified_v4 = addr!("0.0.0.0:8378");
    let unspecified_v6 = addr!("[::]:8378");

    run_test(loopback_v4, false, |addrs| addrs == [loopback_v4]).await?;

    run_test(loopback_v6, false, |addrs| addrs == [loopback_v4]).await?;

    run_test(loopback_v6, true, |addrs| {
        addrs.sort();
        addrs == [loopback_v4, loopback_v6]
    })
    .await?;

    run_test(unspecified_v4, false, |addrs| {
        addrs.len() > 1 && addrs.contains(&loopback_v4)
    })
    .await?;

    run_test(unspecified_v6, false, |addrs| {
        addrs.len() > 1 && addrs.contains(&loopback_v4)
    })
    .await?;

    run_test(unspecified_v6, true, |addrs| {
        addrs.len() > 2 && addrs.contains(&loopback_v4) && addrs.contains(&loopback_v6)
    })
    .await?;

    Ok(())
}
