use super::Device;
use macro_rules_attribute::apply;
use crate::{ASCOMError, ASCOMResult};

/// Rotator Specific Methods.
#[apply(rpc_trait)]
pub trait Rotator: Device + Send + Sync {
    /// True if the Rotator supports the Reverse method.
    #[http("canreverse", method = Get)]
    async fn can_reverse(&self) -> ASCOMResult<bool> {
        Ok(false)
    }

    /// True if the rotator is currently moving to a new position.
    ///
    /// False if the focuser is stationary.
    #[http("ismoving", method = Get)]
    async fn is_moving(&self) -> ASCOMResult<bool> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns the raw mechanical position of the rotator in degrees.
    #[http("mechanicalposition", method = Get)]
    async fn mechanical_position(&self) -> ASCOMResult<f64> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Current instantaneous Rotator position, in degrees.
    #[http("position", method = Get)]
    async fn position(&self) -> ASCOMResult<f64> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns the rotator’s Reverse state.
    #[http("reverse", method = Get)]
    async fn reverse(&self) -> ASCOMResult<bool> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Sets the rotator’s Reverse state.
    #[http("reverse", method = Put)]
    async fn set_reverse(&self, #[http("Reverse")] reverse: bool) -> ASCOMResult {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// The minimum StepSize, in degrees.
    #[http("stepsize", method = Get)]
    async fn step_size(&self) -> ASCOMResult<f64> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// The destination position angle for Move() and MoveAbsolute().
    #[http("targetposition", method = Get)]
    async fn target_position(&self) -> ASCOMResult<f64> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Immediately stop any Rotator motion due to a previous Move or MoveAbsolute method call.
    #[http("halt", method = Put)]
    async fn halt(&self) -> ASCOMResult {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Causes the rotator to move Position degrees relative to the current Position value.
    #[http("move", method = Put)]
    async fn move_(&self, #[http("Position")] position: f64) -> ASCOMResult {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Causes the rotator to move the absolute position of Position degrees.
    #[http("moveabsolute", method = Put)]
    async fn move_absolute(&self, #[http("Position")] position: f64) -> ASCOMResult {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Causes the rotator to move the mechanical position of Position degrees.
    #[http("movemechanical", method = Put)]
    async fn move_mechanical(&self, #[http("Position")] position: f64) -> ASCOMResult {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Causes the rotator to sync to the position of Position degrees.
    #[http("sync", method = Put)]
    async fn sync(&self, #[http("Position")] position: f64) -> ASCOMResult {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// This method returns the version of the ASCOM device interface contract to which this device complies.
    ///
    /// Only one interface version is current at a moment in time and all new devices should be built to the latest interface version. Applications can choose which device interface versions they support and it is in their interest to support  previous versions as well as the current version to ensure thay can use the largest number of devices.
    #[http("interfaceversion", method = Get)]
    async fn interface_version(&self) -> ASCOMResult<i32> {
        Ok(4_i32)
    }
}
