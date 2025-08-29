use super::Device;
use crate::{ASCOMError, ASCOMResult};
use macro_rules_attribute::apply;

/// Focuser Specific Methods.
#[apply(rpc_trait)]
pub trait Focuser: Device + Send + Sync {
    /// True if the focuser is capable of absolute position; that is, being commanded to a specific step location.
    #[http("absolute", method = Get)]
    async fn absolute(&self) -> ASCOMResult<bool>;

    /// True if the focuser is currently moving to a new position.
    ///
    /// False if the focuser is stationary.
    #[http("ismoving", method = Get)]
    async fn is_moving(&self) -> ASCOMResult<bool>;

    /// Maximum increment size allowed by the focuser; i.e. the maximum number of steps allowed in one move operation.
    #[http("maxincrement", method = Get)]
    async fn max_increment(&self) -> ASCOMResult<i32>;

    /// Maximum step position permitted.
    #[http("maxstep", method = Get)]
    async fn max_step(&self) -> ASCOMResult<i32>;

    /// Current focuser position, in steps.
    #[http("position", method = Get)]
    async fn position(&self) -> ASCOMResult<i32> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Step size (microns) for the focuser.
    #[http("stepsize", method = Get)]
    async fn step_size(&self) -> ASCOMResult<f64> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Gets the state of temperature compensation mode (if available), else always False.
    #[http("tempcomp", method = Get)]
    async fn temp_comp(&self) -> ASCOMResult<bool>;

    /// Sets the state of temperature compensation mode.
    #[http("tempcomp", method = Put)]
    async fn set_temp_comp(&self, #[http("TempComp")] temp_comp: bool) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// True if focuser has temperature compensation available.
    #[http("tempcompavailable", method = Get)]
    async fn temp_comp_available(&self) -> ASCOMResult<bool>;

    /// Current ambient temperature as measured by the focuser.
    #[http("temperature", method = Get)]
    async fn temperature(&self) -> ASCOMResult<f64> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Immediately stop any focuser motion due to a previous Move(Int32) method call.
    #[http("halt", method = Put)]
    async fn halt(&self) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Starts moving the focuser by the specified amount or to the specified position depending on the value of the Absolute property.
    #[http("move", method = Put)]
    async fn move_(&self, #[http("Position")] position: i32) -> ASCOMResult<()>;

    /// This method returns the version of the ASCOM device interface contract to which this device complies.
    ///
    /// Only one interface version is current at a moment in time and all new devices should be built to the latest interface version. Applications can choose which device interface versions they support and it is in their interest to support  previous versions as well as the current version to ensure thay can use the largest number of devices.
    #[http("interfaceversion", method = Get)]
    async fn interface_version(&self) -> ASCOMResult<i32> {
        Ok(4_i32)
    }
}
