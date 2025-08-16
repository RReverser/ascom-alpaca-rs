use serde::{Deserialize, Serialize};
use std::borrow::Cow;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ConfiguredDevice<DeviceType> {
    #[serde(rename = "DeviceName")]
    pub(crate) name: String,
    #[serde(rename = "DeviceType")]
    pub(crate) ty: DeviceType,
    #[serde(rename = "DeviceNumber")]
    pub(crate) number: usize,
    #[serde(rename = "UniqueID")]
    pub(crate) unique_id: String,
}

/// General information about the server.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ServerInfo {
    /// Server name.
    pub server_name: Cow<'static, str>,
    /// Manufacturer name.
    pub manufacturer: Cow<'static, str>,
    /// Manufacturer version.
    pub manufacturer_version: Cow<'static, str>,
    /// Server location.
    pub location: Cow<'static, str>,
}

// Using macro namespacing hack from https://users.rust-lang.org/t/how-to-namespace-a-macro-rules-macro-within-a-module-or-macro-export-it-without-polluting-the-top-level-namespace/63779/5?u=rreverser.
#[doc(hidden)]
#[macro_export]
macro_rules! CargoServerInfo_1bc8c806_8cb9_4aaf_b57a_8f94c4d1b59d {
    () => {
        const {
            use std::borrow::Cow;

            $crate::api::ServerInfo {
                server_name: Cow::Borrowed(env!("CARGO_PKG_NAME")),
                manufacturer: Cow::Borrowed(env!("CARGO_PKG_AUTHORS")),
                manufacturer_version: Cow::Borrowed(env!("CARGO_PKG_VERSION")),
                location: {
                    // Technically this field should be a physical location,
                    // but repository homepage seems better than nothing.
                    let homepage = env!("CARGO_PKG_HOMEPAGE");
                    Cow::Borrowed(if homepage.is_empty() {
                        "Unknown"
                    } else {
                        homepage
                    })
                },
            }
        }
    };
}

/// A helper that constructs a [`ServerInfo`](crate::api::ServerInfo) instance populated with metadata from `Cargo.toml`.
#[doc(inline)]
pub use CargoServerInfo_1bc8c806_8cb9_4aaf_b57a_8f94c4d1b59d as CargoServerInfo;
