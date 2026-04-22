//! Discovery of Alpaca devices on the local network.

#[cfg(feature = "client")]
pub use crate::client::{BoundDiscoveryClient, DiscoveryClient};
#[cfg(feature = "server")]
pub use crate::server::{BoundDiscoveryServer, DiscoveryServer};
use serde::{Deserialize, Serialize};
use socket2::{Domain, Protocol, Socket, Type};
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};
#[cfg(windows)]
use std::os::windows::prelude::AsRawSocket;
use tokio::net::UdpSocket;

/// `ff12::a1:9aca` as per ASCOM Alpaca specification.
pub(crate) const DISCOVERY_ADDR_V6: Ipv6Addr = Ipv6Addr::new(0xff12, 0, 0, 0, 0, 0, 0xa1, 0x9aca);
pub(crate) const DISCOVERY_MSG: &[u8] = b"alpacadiscovery1";
pub(crate) const DEFAULT_DISCOVERY_PORT: u16 = 32227;

#[derive(Serialize, Deserialize)]
pub(crate) struct AlpacaPort {
    #[serde(rename = "AlpacaPort")]
    pub(crate) alpaca_port: u16,
}

/// IPv4 address information for a network interface.
#[derive(Debug)]
pub(crate) struct Ipv4Info {
    pub(crate) addr: Ipv4Addr,
    pub(crate) netmask: Ipv4Addr,
}

/// A network interface with all its IPv4 and IPv6 addresses grouped together.
///
/// `ipv6_index` is populated only from V6 entries: on Windows, the V4 and V6
/// interface indices for the same adapter can differ (`IfIndex` vs
/// `Ipv6IfIndex` in `IP_ADAPTER_ADDRESSES`), and the IPv6 multicast APIs
/// (`IPV6_MULTICAST_IF`, `IPV6_JOIN_GROUP`) need the V6-specific one. We don't
/// store a V4 index because IPv4 discovery uses subnet-directed broadcast,
/// where the routing table picks the outbound interface by destination.
#[derive(Debug)]
pub(crate) struct GroupedInterface {
    pub(crate) name: String,
    pub(crate) ipv6_index: Option<u32>,
    pub(crate) is_loopback: bool,
    pub(crate) ipv4: Vec<Ipv4Info>,
    pub(crate) ipv6: Vec<Ipv6Addr>,
}

pub(crate) fn get_active_interfaces() -> eyre::Result<Vec<GroupedInterface>> {
    let mut interfaces: Vec<GroupedInterface> = Vec::new();

    for iface in if_addrs::get_if_addrs()? {
        if !iface.is_oper_up() {
            continue;
        }

        let grouped = if let Some(existing) = interfaces.iter_mut().find(|g| g.name == iface.name)
        {
            existing.is_loopback |= iface.is_loopback();
            existing
        } else {
            interfaces.push(GroupedInterface {
                name: iface.name.clone(),
                ipv6_index: None,
                is_loopback: iface.is_loopback(),
                ipv4: Vec::new(),
                ipv6: Vec::new(),
            });
            interfaces
                .last_mut()
                .expect("internal error: just pushed an element")
        };

        match iface.addr {
            if_addrs::IfAddr::V4(v4) => {
                grouped.ipv4.push(Ipv4Info {
                    addr: v4.ip,
                    netmask: v4.netmask,
                });
            }
            if_addrs::IfAddr::V6(v6) => {
                grouped.ipv6_index = grouped.ipv6_index.or(iface.index);
                grouped.ipv6.push(v6.ip);
            }
        }
    }

    Ok(interfaces)
}

#[tracing::instrument(level = "trace")]
pub(crate) fn bind_socket(addr: SocketAddr) -> eyre::Result<UdpSocket> {
    let socket = Socket::new(Domain::for_address(addr), Type::DGRAM, Some(Protocol::UDP))?;
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
            SIO_UDP_CONNRESET, WSAGetLastError, ioctlsocket,
        };

        unsafe {
            #[expect(
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
    socket.bind(&addr.into())?;
    Ok(UdpSocket::from_std(socket.into())?)
}

#[cfg(test)]
mod tests {
    use super::{DiscoveryClient, DiscoveryServer};
    use futures::StreamExt;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
    use std::sync::LazyLock;

    const TEST_ALPACA_PORT: u16 = 8378;

    #[derive(Default)]
    #[expect(clippy::struct_excessive_bools)]
    struct ExpectedAddrs {
        localhost_v4: bool,
        localhost_v6: bool,
        default_v4: bool,
        default_v6: bool,
    }

    struct DefaultAddr {
        v4: Ipv4Addr,
        v6: Ipv6Addr,
    }

    // TODO: remove when official method is stabilized.
    const fn is_unicast_link_local(ipv6: &Ipv6Addr) -> bool {
        (ipv6.segments()[0] & 0xffc0) == 0xfe80
    }

    /// Determine the default IPv4 address using the UDP socket trick:
    /// "connect" to a non-routable address and read back the source IP the OS chose.
    ///
    /// Uses a TEST-NET-1 address (RFC 5737). VPNs commonly push routes for
    /// RFC1918 prefixes, which would make a `10.0.0.0/8` target resolve to
    /// the VPN interface instead of the true default route.
    fn get_default_ipv4() -> Option<Ipv4Addr> {
        let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
        socket.connect("192.0.2.1:1").ok()?;
        match socket.local_addr().ok()?.ip() {
            IpAddr::V4(ip) => Some(ip),
            IpAddr::V6(_) => None,
        }
    }

    static DEFAULT_ADDR: LazyLock<DefaultAddr> = LazyLock::new(|| {
        // Use the UDP socket trick to find the default route's IPv4, matching
        // the old netdev::get_default_interface() behaviour.
        let default_ip = get_default_ipv4().expect("no default IPv4 route");

        // Find which interface owns that IP, then grab its link-local IPv6 too.
        // This guarantees both addresses come from the same interface.
        let addrs = if_addrs::get_if_addrs().expect("couldn't get network interfaces");
        let default_intf_name = &addrs
            .iter()
            .find(|iface| iface.ip() == IpAddr::V4(default_ip))
            .expect("default IP not found on any interface")
            .name;

        DefaultAddr {
            v4: default_ip,
            v6: addrs
                .iter()
                .filter(|iface| &iface.name == default_intf_name)
                .filter_map(|iface| match &iface.addr {
                    if_addrs::IfAddr::V6(v6) => Some(v6.ip),
                    if_addrs::IfAddr::V4(_) => None,
                })
                .find(is_unicast_link_local)
                .expect("no link-local IPv6 on default interface"),
        }
    });

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
                        (IpAddr::from(DEFAULT_ADDR.v4), expected_addrs.default_v4),
                        (IpAddr::from(DEFAULT_ADDR.v6), expected_addrs.default_v6),
                    ];

                for (addr, expected) in expected_addrs {
                    eyre::ensure!(addrs.contains(&addr) == expected, "Address {addr} was{not} expected in {addrs:#?}", not = if expected { "" } else { " not" });
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
        test_unspecified_v4 = Ipv4Addr::UNSPECIFIED => localhost_v4, default_v4;
        test_unspecified_v6 = Ipv6Addr::UNSPECIFIED => localhost_v4, localhost_v6, default_v4, default_v6;
        test_external_v4 = DEFAULT_ADDR.v4 => default_v4;
        test_external_v6 = DEFAULT_ADDR.v6 => default_v6;
    }

    /// IPv4-only discovery test suitable for CI environments that lack a
    /// default route or link-local IPv6 on the default interface.
    ///
    /// Unlike the tests above, this does not touch `DEFAULT_ADDR`, so it
    /// doesn't require interface enumeration to find a link-local IPv6. The
    /// server binds to `0.0.0.0` so that loopback subnet broadcasts
    /// (`127.255.255.255`) reach it regardless of OS-specific loopback
    /// routing behavior, and the server skips all multicast joins because
    /// the listen address is IPv4.
    #[tokio::test]
    async fn test_unspecified_v4_only() -> eyre::Result<()> {
        let server_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), TEST_ALPACA_PORT);
        let mut server = DiscoveryServer::for_alpaca_server_at(server_addr);
        server.listen_addr.set_port(0);

        let bound_server = server.bind().await?;

        let client = DiscoveryClient {
            discovery_port: bound_server.listen_addr().port(),
            ..Default::default()
        };

        tokio::select! {
            never_returns = bound_server.start() => match never_returns {},

            result = async {
                let addrs = client
                    .bind()
                    .await?
                    .discover_addrs()
                    .collect::<Vec<_>>()
                    .await;

                let has_v4 = addrs
                    .iter()
                    .any(|addr| addr.port() == TEST_ALPACA_PORT && addr.is_ipv4());

                eyre::ensure!(
                    has_v4,
                    "expected at least one IPv4 discovered addr, got {addrs:?}"
                );

                Ok::<_, eyre::Error>(())
            } => result,
        }
    }

    /// IPv6-loopback-only discovery test suitable for CI environments
    /// that lack a link-local IPv6 on the default interface.
    ///
    /// Exercises the IPv6 code paths: server bind to `::1` (goes through
    /// `join_multicast_groups`' "specific address" branch), client sending
    /// V6 unicast to the loopback `::1`, and V6 response handling.
    #[tokio::test]
    async fn test_loopback_v6_only() -> eyre::Result<()> {
        let server_addr = SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), TEST_ALPACA_PORT);
        let mut server = DiscoveryServer::for_alpaca_server_at(server_addr);
        server.listen_addr.set_port(0);

        let bound_server = server.bind().await?;

        let client = DiscoveryClient {
            discovery_port: bound_server.listen_addr().port(),
            ..Default::default()
        };

        tokio::select! {
            never_returns = bound_server.start() => match never_returns {},

            result = async {
                let addrs = client
                    .bind()
                    .await?
                    .discover_addrs()
                    .collect::<Vec<_>>()
                    .await;

                let has_v6_loopback = addrs.iter().any(|addr| {
                    addr.port() == TEST_ALPACA_PORT
                        && addr.ip() == IpAddr::V6(Ipv6Addr::LOCALHOST)
                });

                eyre::ensure!(
                    has_v6_loopback,
                    "expected ::1 discovered addr, got {addrs:?}"
                );

                Ok::<_, eyre::Error>(())
            } => result,
        }
    }
}
