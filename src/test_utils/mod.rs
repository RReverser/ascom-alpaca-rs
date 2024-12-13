#[cfg(test)]
mod logging_env;

macro_rules! cmd {
    ($windows_path_hint:literal, $name:literal) => {
        tokio::process::Command::new(if cfg!(windows) {
            static ADD_COMMON_PATH: std::sync::Once = std::sync::Once::new();

            ADD_COMMON_PATH.call_once(|| {
                let mut path = std::env::var_os("PATH").unwrap_or_default();
                path.push(concat!(";", $windows_path_hint));
                unsafe {
                    std::env::set_var("PATH", path);
                }
            });

            concat!($name, ".exe")
        } else {
            $name
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
