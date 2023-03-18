use net_literals::ipv6;
use serde::{Deserialize, Serialize};
use std::net::Ipv6Addr;

const DISCOVERY_ADDR_V6: Ipv6Addr = ipv6!("ff12::a1:9aca");
const DISCOVERY_MSG: &[u8] = b"alpacadiscovery1";

pub const DEFAULT_DISCOVERY_PORT: u16 = 32227;

#[derive(Serialize, Deserialize)]
struct AlpacaPort {
    #[serde(rename = "AlpacaPort")]
    alpaca_port: u16,
}

mod client;
mod server;

pub use client::Client;
pub use server::Server;
