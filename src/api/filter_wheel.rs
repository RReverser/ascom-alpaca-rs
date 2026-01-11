use super::Device;
use crate::{ASCOMError, ASCOMResult};
use macro_rules_attribute::apply;

/// FilterWheel Specific Methods.
#[apply(rpc_trait)]
pub trait FilterWheel: Device + Send + Sync {
    /// An integer array of filter focus offsets.
    #[http("focusoffsets", method = Get)]
    async fn focus_offsets(&self) -> ASCOMResult<Vec<i32>>;

    /// The names of the filters.
    #[http("names", method = Get)]
    async fn names(&self) -> ASCOMResult<Vec<String>>;

    /// Returns the current filter wheel position.
    #[http("position", method = Get, device_state = "Position")]
    async fn position(&self) -> ASCOMResult<usize>;

    /// Sets the filter wheel position.
    #[http("position", method = Put)]
    async fn set_position(&self, #[http("Position")] position: usize) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// This method returns the version of the ASCOM device interface contract to which this device complies.
    ///
    /// Only one interface version is current at a moment in time and all new devices should be built to the latest interface version. Applications can choose which device interface versions they support and it is in their interest to support  previous versions as well as the current version to ensure thay can use the largest number of devices.
    #[http("interfaceversion", method = Get)]
    async fn interface_version(&self) -> ASCOMResult<u16> {
        Ok(3)
    }
}
