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

pub(crate) fn get_active_interfaces() -> impl Iterator<Item = Interface> {
    netdev::get_interfaces()
        .into_iter()
        .filter(Interface::is_running)
}

#[tracing::instrument(level = "trace")]
pub(crate) async fn bind_socket(
    addr: impl Into<SocketAddr> + std::fmt::Debug + Send,
) -> eyre::Result<tokio::net::UdpSocket> {
    let addr = addr.into();
    let socket = socket2::Socket::new(
        socket2::Domain::for_address(addr),
        socket2::Type::DGRAM,
        Some(socket2::Protocol::UDP),
    )?;
    // For async code, we need to set the socket to non-blocking mode.
    socket.set_nonblocking(true)?;
    // Reuse address for parallel server instances in e.g. tests.
    socket.set_reuse_address(true)?;
    if addr.is_ipv6() {
        // We want to talk to the IPv4 broadcast address from the same socket.
        // Using `socket2` seems to be the only way to do this from safe Rust.
        socket.set_only_v6(false)?;
    }
    // SIO_UDP_CONNRESET is needed to ignore the occasional "port unreachable" errors
    // on Windows. Ideally we'd just ignore the error and move on but those tend to
    // render socket unusable so we'd have to recreate it as well.
    #[cfg(windows)]
    {
        use eyre::Context;
        use windows_sys::Win32::Networking::WinSock::{
            ioctlsocket, WSAGetLastError, SIO_UDP_CONNRESET,
        };

        unsafe {
            #[allow(
                clippy::as_conversions,
                clippy::cast_possible_truncation,
                clippy::cast_possible_wrap
            )]
            match ioctlsocket(socket.as_raw_socket() as _, SIO_UDP_CONNRESET as _, &mut 0) {
                0_i32 => Ok(()),
                _ => Err(WSAGetLastError()),
            }
        }
        .map_err(std::io::Error::from_raw_os_error)
        .context("Couldn't configure the UDP socket to ignore ICMP errors")?;
    }
    let socket = tokio::task::spawn_blocking(move || {
        socket.bind(&addr.into())?;
        Ok::<_, eyre::Error>(socket)
    })
    .await??;
    Ok(tokio::net::UdpSocket::from_std(socket.into())?)
}

#[cfg(feature = "client")]
pub use crate::client::{BoundDiscoveryClient, DiscoveryClient};
#[cfg(feature = "server")]
pub use crate::server::{BoundDiscoveryServer, DiscoveryServer};
use netdev::Interface;
#[cfg(windows)]
use std::os::windows::prelude::AsRawSocket;

#[cfg(test)]
#[serial_test::serial(discovery)]
mod tests {
    use super::{DiscoveryClient, DiscoveryServer};
    use futures::StreamExt;
    use once_cell::sync::Lazy;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

    const TEST_ALPACA_PORT: u16 = 8378;

    #[derive(Default)]
    struct ExpectedAddrs {
        localhost_v4: bool,
        localhost_v6: bool,
        default_intf_v4: bool,
        default_intf_v6: bool,
    }

    static DEFAULT_INTF: Lazy<netdev::Interface> =
        Lazy::new(|| netdev::get_default_interface().expect("coudn't get default interface"));

    async fn run_test(server_addr: IpAddr, expected_addrs: ExpectedAddrs) -> eyre::Result<()> {
        let mut server =
            DiscoveryServer::for_alpaca_server_at(SocketAddr::new(server_addr, TEST_ALPACA_PORT));

        // override discovery server port with a random one so that tests are independent from each other
        server.listen_addr.set_port(0);

        let bound_server = server.bind().await?;

        let client = DiscoveryClient {
            discovery_port: bound_server.listen_addr().port(),
            ..Default::default()
        };

        tokio::select! {
            never_returns = bound_server.start() => match never_returns {},

            addrs = async {
                Ok::<_, eyre::Error>(
                    client
                    .bind()
                    .await?
                    .discover_addrs()
                    .collect::<Vec<_>>()
                    .await
                )
             } => {
                let addrs = addrs?.iter().filter(|addr|
                    // Filter out unrelated servers potentially running in background.
                    addr.port() == TEST_ALPACA_PORT
                ).map(SocketAddr::ip).collect::<Vec<_>>();

                if server_addr.is_ipv4() {
                    eyre::ensure!(addrs.iter().all(IpAddr::is_ipv4), "IPv4 server can't be discovered via IPv6 address");
                }

                // Collect all expected addresses -> bool pairs so that we can check for both expected and unexpected addresses.
                let expected_addrs =
                    [
                        (IpAddr::from(Ipv4Addr::LOCALHOST), expected_addrs.localhost_v4),
                        (IpAddr::from(Ipv6Addr::LOCALHOST), expected_addrs.localhost_v6),
                    ].into_iter()
                    .chain(DEFAULT_INTF.ipv4.iter().map(|net| (net.addr.into(), expected_addrs.default_intf_v4)))
                    .chain(DEFAULT_INTF.ipv6.iter().map(|net| (net.addr.into(), expected_addrs.default_intf_v6)));

                for (addr, expected) in expected_addrs {
                    eyre::ensure!(addrs.contains(&addr) == expected, "Address {addr} was{not} expected", not = if expected { "" } else { " not" });
                }

                Ok(())
            }
        }
    }

    macro_rules! declare_tests {
        ($($name:ident = $addr:expr => $($expected_addrs:ident),+;)*) => {
            $(
                #[tokio::test]
                async fn $name() -> eyre::Result<()> {
                    run_test(
                        $addr.into(),
                        #[allow(clippy::needless_update)]
                        ExpectedAddrs { $($expected_addrs: true,)+ ..Default::default() }
                    ).await
                }
            )*
        };
    }

    declare_tests! {
        test_loopback_v4 = Ipv4Addr::LOCALHOST => localhost_v4;
        test_loopback_v6 = Ipv6Addr::LOCALHOST => localhost_v6;
        test_unspecified_v4 = Ipv4Addr::UNSPECIFIED => localhost_v4, default_intf_v4;
        test_unspecified_v6 = Ipv6Addr::UNSPECIFIED => localhost_v4, localhost_v6, default_intf_v4, default_intf_v6;
        test_external_v4 = DEFAULT_INTF.ipv4[0].addr => default_intf_v4;
        test_external_v6 = DEFAULT_INTF.ipv6[0].addr => default_intf_v6;
    }
}
