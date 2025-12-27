use super::Device;
use crate::ASCOMResult;
use macro_rules_attribute::apply;

/// SafetyMonitor Specific Methods.
#[apply(rpc_trait)]
pub trait SafetyMonitor: Device + Send + Sync {
    /// Indicates whether the monitored state is safe for use.
    ///
    /// True if the state is safe, False if it is unsafe.
    #[http("issafe", method = Get, device_state = "IsSafe")]
    async fn is_safe(&self) -> ASCOMResult<bool>;

    /// This method returns the version of the ASCOM device interface contract to which this device complies.
    ///
    /// Only one interface version is current at a moment in time and all new devices should be built to the latest interface version. Applications can choose which device interface versions they support and it is in their interest to support  previous versions as well as the current version to ensure thay can use the largest number of devices.
    #[http("interfaceversion", method = Get)]
    async fn interface_version(&self) -> ASCOMResult<i32> {
        Ok(3_i32)
    }
}
