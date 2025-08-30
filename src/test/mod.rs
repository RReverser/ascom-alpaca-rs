use std::path::PathBuf;

#[cfg(test)]
mod logging_env;

#[cfg(feature = "client")]
pub use crate::client::test::get_simulator_devices;

#[cfg(feature = "server")]
pub use crate::server::test::run_conformu_tests;

pub(crate) fn resolve_path(path_hint: &'static str, exe_name: &'static str) -> PathBuf {
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
            use std::sync::LazyLock;
            // On Windows, ASCOM binaries have well-known path that we can look up if executable is not on the global PATH.
            static RESOLVED_PATH: LazyLock<std::path::PathBuf> = LazyLock::new(|| {
                $crate::test::resolve_path($windows_path_hint, concat!($name, ".exe"))
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

#[cfg(test)]
pub(crate) async fn run_proxy_tests<T: ?Sized + crate::api::RetrieavableDevice>() -> eyre::Result<()>
{
    use crate::Server;
    use crate::api::CargoServerInfo;
    use crate::client::test::get_simulator_devices;
    use crate::server::test::run_conformu_tests;
    use net_literals::addr;

    let proxy = Server {
        devices: get_simulator_devices().await?.clone(),
        listen_addr: addr!("127.0.0.1:0"),
        ..Server::new(CargoServerInfo!())
    };

    let proxy = proxy.bind().await?;

    // Get the IP and the random port assigned by the OS.
    let server_url = format!("http://{}/", proxy.listen_addr());

    tokio::select! {
        proxy_result = proxy.start() => match proxy_result? {},
        tests_result = run_conformu_tests::<T>(server_url, 0) => tests_result,
    }
}
