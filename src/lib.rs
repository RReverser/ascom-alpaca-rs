#[path = "autogen/mod.rs"]
pub mod api;

mod devices;
mod errors;
mod rpc;
mod transaction;

pub use devices::{Devices, DevicesBuilder};
pub use errors::{ASCOMError, ASCOMErrorCode, ASCOMResult};
pub use rpc::OpaqueResponse;
pub use transaction::respond_with;

#[cfg(feature = "actix")]
mod actix;
