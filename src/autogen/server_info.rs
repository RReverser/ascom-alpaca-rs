use super::DeviceType;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ConfiguredDevice {
    pub device_name: String,
    pub device_type: DeviceType,
    pub device_number: usize,
    #[serde(rename = "UniqueID")]
    pub unique_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ServerInfo {
    pub server_name: String,
    pub manufacturer: String,
    pub manufacturer_version: String,
    pub location: String,
}

// Using macro namespacing hack from https://users.rust-lang.org/t/how-to-namespace-a-macro-rules-macro-within-a-module-or-macro-export-it-without-polluting-the-top-level-namespace/63779/5?u=rreverser.
#[doc(hidden)]
#[macro_export]
macro_rules! CargoServerInfo_1bc8c806_8cb9_4aaf_b57a_8f94c4d1b59d {
    () => {
        $crate::api::ServerInfo {
            server_name: env!("CARGO_PKG_NAME").to_owned(),
            manufacturer: env!("CARGO_PKG_AUTHORS").to_owned(),
            manufacturer_version: env!("CARGO_PKG_VERSION").to_owned(),
            location: env!("CARGO_PKG_HOMEPAGE").to_owned(),
        }
    };
}

#[doc(inline)]
pub use CargoServerInfo_1bc8c806_8cb9_4aaf_b57a_8f94c4d1b59d as CargoServerInfo;
