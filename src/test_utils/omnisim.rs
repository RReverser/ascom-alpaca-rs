use crate::{Client, Devices};
use net_literals::addr;
use std::net::SocketAddr;
use std::process::Stdio;
use std::sync::{Arc, Weak};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

/// A helper that manages [ASCOM Alpaca Simulators](https://github.com/ASCOMInitiative/ASCOM.Alpaca.Simulators).
///
/// Acquiring this helper via [`acquire`](`Self::acquire`) ensures that the simulators process is running in the background (either by launching it or reusing an existing instance).
/// This is helpful for client integration tests that require the simulators to be running.
///
/// You can retrieve the device clients exposed by the simulators via the [`devices`](`Self::devices`) method.
#[derive(custom_debug::Debug)]
pub struct OmniSim {
    #[debug(skip)]
    _server: Child,
    devices: Devices,
}

impl OmniSim {
    async fn new() -> eyre::Result<Self> {
        const ADDR: SocketAddr = addr!("127.0.0.1:32323");

        let mut server =
            Command::new(r"C:\Program Files\ASCOM\OmniSimulator\ascom.alpaca.simulators.exe")
                .arg(format!("--urls=http://{ADDR}"))
                .arg("--set-no-browser")
                .stdin(Stdio::null())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .kill_on_drop(true)
                .spawn()?;

        tokio::select! {
            () = async {
                while tokio::net::TcpStream::connect(ADDR).await.is_err() {
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                }
            } => {}
            server_exited = server.wait() => eyre::bail!("Simulator process exited early: {}", server_exited?),
            () = tokio::time::sleep(std::time::Duration::from_secs(10)) => eyre::bail!("Simulator process didn't start in time")
        }

        Ok(Self {
            _server: server,
            devices: Client::new_from_addr(ADDR).get_devices().await?.collect(),
        })
    }

    /// Get or create a shared instance of the test environment.
    ///
    /// Note that the simulators process is stopped when the last instance of this helper is dropped - make sure to keep it alive for the duration of the tests.
    pub async fn acquire() -> eyre::Result<Arc<Self>> {
        // Note: the static variable should only contain a Weak copy, otherwise the test environment
        // would never be dropped, and we want it to be dropped at the end of the last strong copy
        // (last running test).
        static TEST_ENV: Mutex<Weak<OmniSim>> = Mutex::const_new(Weak::new());

        let mut lock = TEST_ENV.lock().await;

        Ok(match lock.upgrade() {
            Some(env) => env,
            None => {
                let env = Arc::new(Self::new().await?);
                *lock = Arc::downgrade(&env);
                env
            }
        })
    }

    /// Get the devices exposed by the simulators.
    pub const fn devices(&self) -> &Devices {
        &self.devices
    }
}
