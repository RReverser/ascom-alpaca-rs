use std::path::PathBuf;

#[cfg(test)]
mod logging_env;

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

#[cfg(feature = "server")]
mod conformu;
#[cfg(feature = "server")]
pub use conformu::run_tests as run_conformu_tests;

#[cfg(feature = "client")]
mod omnisim;
#[cfg(feature = "client")]
pub use omnisim::get_devices as get_simulator_devices;

#[cfg(test)]
pub(crate) async fn run_proxy_tests<T: ?Sized + crate::api::RetrieavableDevice>() -> eyre::Result<()> {
    use crate::Server;
    use net_literals::addr;

    let proxy = Server {
        devices: get_simulator_devices().await?.clone(),
        listen_addr: addr!("127.0.0.1:0"),
        ..Default::default()
    };

    let proxy = proxy.bind().await?;

    let device_url = format!(
        "http://{listen_addr}/api/v1/{ty}/0",
        // Get the IP and the random port assigned by the OS.
        listen_addr = proxy.listen_addr(),
        ty = T::TYPE
    );

    tokio::select! {
        proxy_result = proxy.start() => match proxy_result? {},
        tests_result = run_conformu_tests::<T>(&device_url) => tests_result,
    }
}
