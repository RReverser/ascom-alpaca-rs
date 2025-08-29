use super::Device;
use macro_rules_attribute::apply;
use crate::{ASCOMError, ASCOMResult};

/// FilterWheel Specific Methods.
#[apply(rpc_trait)]
pub trait FilterWheel: Device + Send + Sync {
    /// An integer array of filter focus offsets.
    #[http("focusoffsets", method = Get)]
    async fn focus_offsets(&self) -> ASCOMResult<Vec<i32>> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// The names of the filters.
    #[http("names", method = Get)]
    async fn names(&self) -> ASCOMResult<Vec<String>> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns the current filter wheel position.
    #[http("position", method = Get)]
    async fn position(&self) -> ASCOMResult<i32> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Sets the filter wheel position.
    #[http("position", method = Put)]
    async fn set_position(&self, #[http("Position")] position: i32) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// This method returns the version of the ASCOM device interface contract to which this device complies.
    ///
    /// Only one interface version is current at a moment in time and all new devices should be built to the latest interface version. Applications can choose which device interface versions they support and it is in their interest to support  previous versions as well as the current version to ensure thay can use the largest number of devices.
    #[http("interfaceversion", method = Get)]
    async fn interface_version(&self) -> ASCOMResult<i32> {
        Ok(3_i32)
    }
}
