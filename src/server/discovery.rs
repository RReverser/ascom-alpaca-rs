use super::DEFAULT_DISCOVERY_PORT;
use crate::discovery::{AlpacaPort, DISCOVERY_ADDR_V6, DISCOVERY_MSG, bind_socket};

#[derive(Debug, Clone, Copy)]
pub struct Server {
    /// Port of the running server.
    pub alpaca_port: u16,
    /// Discovery port to listen on.
    ///
    /// Defaults to 32227.
    pub discovery_port: u16,
}

impl Server {
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
        let socket = bind_socket(self.discovery_port)?;
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
