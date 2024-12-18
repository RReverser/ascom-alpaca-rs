#[cfg(test)]
mod logging_env;

pub(crate) fn resolve_path(path_hint: &'static str, exe_name: &'static str) -> std::path::PathBuf {
    use std::env;

    env::split_paths(&env::var_os("PATH").unwrap_or_default())
    .chain(std::iter::once(path_hint.into()))
    .map(|path| path.join(exe_name))
    .find(|path| path.exists())
    .unwrap_or_else(|| panic!("{exe_name} not found in either PATH or the standard installation directory {path_hint}"))
}

macro_rules! cmd {
    ($windows_path_hint:literal, $name:literal) => {
        tokio::process::Command::new(if cfg!(windows) {
            // On Windows, ASCOM binaries have well-known path that we can look up if executable is not on the global PATH.
            static RESOLVED_PATH: std::sync::LazyLock<std::path::PathBuf> = std::sync::LazyLock::new(|| {
                $crate::test_utils::resolve_path($windows_path_hint, concat!($name, ".exe"))
            });
            &RESOLVED_PATH
        } else {
            // On other systems, just rely on the user adding binaries to the global PATH.
            std::path::Path::new($name)
        })
        .kill_on_drop(true)
        .stdin(Stdio::null())
    };
}

#[cfg(feature = "server")]
mod conformu;
#[cfg(feature = "server")]
pub use conformu::ConformU;

#[cfg(feature = "client")]
mod omnisim;
#[cfg(feature = "client")]
pub use omnisim::OmniSim;

#[cfg(test)]
impl ConformU {
    pub(crate) async fn run_proxy_test(self, ty: crate::api::DeviceType) -> eyre::Result<()> {
        use crate::api::DevicePath;
        use crate::Server;
        use net_literals::addr;

        let env = OmniSim::acquire().await?;

        let proxy = Server {
            devices: env.devices().clone(),
            listen_addr: addr!("127.0.0.1:0"),
            ..Default::default()
        };

        let proxy = proxy.bind().await?;

        // Get the IP and the random port assigned by the OS.
        let listen_addr = proxy.listen_addr();

        let proxy_task = proxy.start();

        let device_url = format!(
            "http://{listen_addr}/api/v1/{device_path}/0",
            device_path = DevicePath(ty)
        );

        let tests_task = self.run(&device_url);

        tokio::select! {
            proxy_result = proxy_task => match proxy_result? {},
            tests_result = tests_task => tests_result,
        }
    }
}
