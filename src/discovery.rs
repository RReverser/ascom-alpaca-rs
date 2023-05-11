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

#[tracing::instrument(err, level = "debug")]
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
    if addr.is_ipv6() {
        // We want to talk to the IPv4 broadcast address from the same socket.
        // Using `socket2` seems to be the only way to do this from safe Rust.
        socket.set_only_v6(false)?;
    }
    #[cfg(windows)]
    #[allow(clippy::as_conversions, clippy::cast_possible_truncation)]
    unsafe {
        use eyre::Context;
        use windows_sys::Win32::Networking::WinSock::{
            WSAGetLastError, WSAIoctl, SIO_UDP_CONNRESET,
        };

        let input: i32 = 0;
        let mut output: u32 = 0;

        let error_code = WSAIoctl(
            socket.as_raw_socket() as _,
            SIO_UDP_CONNRESET,
            std::ptr::addr_of!(input).cast(),
            std::mem::size_of_val(&input) as _,
            std::ptr::null_mut(),
            0,
            std::ptr::addr_of_mut!(output).cast(),
            std::ptr::null_mut(),
            None,
        );
        if error_code != 0_i32 {
            return Err(std::io::Error::from_raw_os_error(WSAGetLastError()))
                .context("Couldn't configure the UDP socket to ignore ICMP errors");
        }
    }
    let socket = tokio::task::spawn_blocking(move || {
        socket.bind(&addr.into())?;
        Ok::<_, eyre::Error>(socket)
    })
    .await??;
    Ok(tokio::net::UdpSocket::from_std(socket.into())?)
}

#[cfg(feature = "client")]
pub use crate::client::DiscoveryClient;
#[cfg(feature = "server")]
pub use crate::server::DiscoveryServer;
#[cfg(windows)]
use std::os::windows::prelude::AsRawSocket;

#[cfg(test)]
mod tests {
    use super::{DiscoveryClient, DiscoveryServer};
    use futures::StreamExt;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

    const TEST_PORT: u16 = 8378;

    async fn run_test(server_addr: IpAddr) -> eyre::Result<()> {
        let server_task =
            DiscoveryServer::for_alpaca_server_at(SocketAddr::new(server_addr, TEST_PORT))
                .bind()
                .await?
                .start();

        tokio::select! {
            never_returns = server_task => match never_returns {},

            addrs = async {
                Ok::<_, eyre::Error>(DiscoveryClient::new().bind().await?.discover_addrs().collect::<Vec<_>>().await)
             } => {
                let mut addrs = addrs?;
                // Filter out unrelated servers potentially running in background.
                addrs.retain(|addr| addr.port() == TEST_PORT);
                let mut addrs = addrs.iter().map(SocketAddr::ip).collect::<Vec<_>>();

                let mut expected_addrs =
                    tokio::task::spawn_blocking(default_net::get_interfaces).await?.into_iter()
                    .flat_map(|iface| {
                        let v4 = iface.ipv4.into_iter().map(|net| net.addr).filter(|addr| !addr.is_link_local()).map(IpAddr::V4);
                        let v6 = iface.ipv6.into_iter().map(|net| net.addr).map(IpAddr::V6);

                        Iterator::chain(v4, v6)
                    })
                    .filter(|addr| {
                        if server_addr.is_ipv4() && addr.is_ipv6() {
                            // IPv4 server can't be discovered from IPv6 client
                            // (but vice versa can due to dual-stack).
                            return false;
                        }
                        if server_addr.is_unspecified() {
                            // Server listening on unspecified address can be discovered over any IP.
                            return true;
                        }
                        // Otherwise the addresses should match.
                        *addr == server_addr
                    })
                    .collect::<Vec<_>>();

                addrs.sort();
                expected_addrs.sort();
                eyre::ensure!(
                    addrs == expected_addrs,
                    "Discovered addresses (left) don't match the expected ones (right):{}",
                    pretty_assertions::Comparison::new(&addrs, &expected_addrs)
                );
                Ok(())
            }
        }
    }

    macro_rules! declare_tests {
        ($($name:ident = $addr:expr;)*) => {
            $(
                #[tokio::test]
                #[serial_test::serial]
                async fn $name() -> eyre::Result<()> {
                    run_test($addr.into()).await
                }
            )*
        };
    }

    declare_tests! {
        test_loopback_v4 = Ipv4Addr::LOCALHOST;
        test_loopback_v6 = Ipv6Addr::LOCALHOST;
        test_unspecified_v4 = Ipv4Addr::UNSPECIFIED;
        test_unspecified_v6 = Ipv6Addr::UNSPECIFIED;
        test_external_v4 = default_net::get_default_interface().map_err(eyre::Error::msg)?.ipv4[0].addr;
        test_external_v6 = default_net::get_default_interface().map_err(eyre::Error::msg)?.ipv6[0].addr;
    }
}
