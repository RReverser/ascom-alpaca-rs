use crate::{Client, Devices};
use net_literals::addr;
use std::net::SocketAddr;
use std::process::Stdio;
use tokio::net::TcpStream;
use tokio::process::Child;
use tokio::sync::{Mutex, OnceCell};
use tokio::time::{sleep, Duration};

const ADDR: SocketAddr = addr!("127.0.0.1:32323");

struct OmniSim {
    server: Mutex<Child>,
    devices: Devices,
}

static OMNISIM: OnceCell<OmniSim> = OnceCell::const_new();

#[dtor::dtor]
fn kill_server() {
    if let Some(omnisim) = OMNISIM.get() {
        let mut server = omnisim.server.blocking_lock();
        if let Err(err) = server.start_kill() {
            tracing::error!(%err, "Failed to kill the simulator server");
        }
    }
}

/// Get devices for testing from the ASCOM Alpaca simulator.
///
/// This helper starts the simulator in background and returns a cached list
/// of available devices.
///
/// It also registers a destructor so that the simulator process is reliably
/// killed whenever the test process exits.
#[tracing::instrument]
pub async fn get_simulator_devices() -> eyre::Result<&'static Devices> {
    OMNISIM.get_or_try_init(async || {
        let mut server =
            cmd!(
                r"C:\Program Files\ASCOM\OmniSimulator",
                "ascom.alpaca.simulators"
            )
            .arg(format!("--urls=http://{ADDR}"))
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .kill_on_drop(true)
            .spawn()?;

        tokio::select! {
            () = async {
                while TcpStream::connect(ADDR).await.is_err() {
                    sleep(Duration::from_millis(100)).await;
                }
            } => {}
            server_exited = server.wait() => {
                let status = server_exited?;
                // Simulator can exit early if it either fails to start or if the simulator is already running.
                // In the latter case (common with e.g. `cargo nextest` running tests in parallel), don't error out.
                if !status.success() {
                    eyre::bail!("Simulator process exited early: {status}");
                }
            },
            () = sleep(Duration::from_secs(10)) => eyre::bail!("Simulator process didn't start in time")
        }

        Ok(OmniSim {
            server: Mutex::new(server),
            devices: Client::new_from_addr(ADDR).get_devices().await?.collect()
        })
    }).await.map(|omnisim| &omnisim.devices)
}
