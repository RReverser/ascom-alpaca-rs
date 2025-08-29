use super::Device;
use crate::{ASCOMError, ASCOMResult};
use macro_rules_attribute::apply;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use serde_repr::{Deserialize_repr, Serialize_repr};

/// CoverCalibrator Specific Methods.
#[apply(rpc_trait)]
pub trait CoverCalibrator: Device + Send + Sync {
    /// Returns the current calibrator brightness in the range 0 (completely off) to MaxBrightness (fully on).
    #[http("brightness", method = Get, device_state = Brightness)]
    async fn brightness(&self) -> ASCOMResult<i32> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// True if the calibrator is not yet stable.
    ///
    /// _ICoverCalibratorV2 and later._
    #[http("calibratorchanging", method = Get, device_state = CalibratorChanging)]
    async fn calibrator_changing(&self) -> ASCOMResult<bool> {
        Ok(self.calibrator_state().await? == CalibratorStatus::NotReady)
    }

    /// Returns the state of the calibration device, if present, otherwise returns "NotPresent".
    ///
    /// The calibrator state mode is specified as an integer value from the CalibratorStatus Enum.
    #[http("calibratorstate", method = Get, device_state = CalibratorState)]
    async fn calibrator_state(&self) -> ASCOMResult<CalibratorStatus>;

    /// True if the cover is moving.
    ///
    /// _ICoverCalibratorV2 and later._
    #[http("covermoving", method = Get, device_state = CoverMoving)]
    async fn cover_moving(&self) -> ASCOMResult<bool> {
        Ok(self.cover_state().await? == CoverStatus::Moving)
    }

    /// Returns the state of the device cover, if present, otherwise returns "NotPresent".
    ///
    /// The cover state mode is specified as an integer value from the CoverStatus Enum.
    #[http("coverstate", method = Get, device_state = CoverState)]
    async fn cover_state(&self) -> ASCOMResult<CoverStatus>;

    /// The Brightness value that makes the calibrator deliver its maximum illumination.
    #[http("maxbrightness", method = Get)]
    async fn max_brightness(&self) -> ASCOMResult<i32> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Turns the calibrator off if the device has calibration capability.
    #[http("calibratoroff", method = Put)]
    async fn calibrator_off(&self) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Turns the calibrator on at the specified brightness if the device has calibration capability.
    #[http("calibratoron", method = Put)]
    async fn calibrator_on(&self, #[http("Brightness")] brightness: i32) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Initiates cover closing if a cover is present.
    #[http("closecover", method = Put)]
    async fn close_cover(&self) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Stops any cover movement that may be in progress if a cover is present and cover movement can be interrupted.
    #[http("haltcover", method = Put)]
    async fn halt_cover(&self) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Initiates cover opening if a cover is present.
    #[http("opencover", method = Put)]
    async fn open_cover(&self) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// This method returns the version of the ASCOM device interface contract to which this device complies.
    ///
    /// Only one interface version is current at a moment in time and all new devices should be built to the latest interface version. Applications can choose which device interface versions they support and it is in their interest to support  previous versions as well as the current version to ensure thay can use the largest number of devices.
    #[http("interfaceversion", method = Get)]
    async fn interface_version(&self) -> ASCOMResult<i32> {
        Ok(2_i32)
    }
}

/// Describes the state of a calibration device.
#[derive(
    Debug,
    PartialEq,
    Eq,
    Clone,
    Copy,
    Serialize_repr,
    Deserialize_repr,
    TryFromPrimitive,
    IntoPrimitive,
)]
#[repr(i32)]
pub enum CalibratorStatus {
    /// This device does not have a calibration capability.
    NotPresent = 0,

    /// The calibrator is off.
    Off = 1,

    /// The calibrator is stabilising or is not yet in the commanded state.
    NotReady = 2,

    /// The calibrator is ready for use.
    Ready = 3,

    /// The calibrator state is unknown.
    Unknown = 4,

    /// The calibrator encountered an error when changing state.
    Error = 5,
}

/// Describes the state of a telescope cover.
#[derive(
    Debug,
    PartialEq,
    Eq,
    Clone,
    Copy,
    Serialize_repr,
    Deserialize_repr,
    TryFromPrimitive,
    IntoPrimitive,
)]
#[repr(i32)]
pub enum CoverStatus {
    /// This device does not have a cover that can be closed independently.
    NotPresent = 0,

    /// The cover is closed.
    Closed = 1,

    /// The cover is moving to a new position.
    Moving = 2,

    /// The cover is open.
    Open = 3,

    /// The state of the cover is unknown.
    Unknown = 4,

    /// The device encountered an error when changing state.
    Error = 5,
}
