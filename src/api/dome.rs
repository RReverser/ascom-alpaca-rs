use super::Device;
use macro_rules_attribute::apply;
use crate::{ASCOMError, ASCOMResult};
use serde_repr::{Deserialize_repr, Serialize_repr};
use num_enum::{IntoPrimitive, TryFromPrimitive};

/// Dome Specific Methods.
#[apply(rpc_trait)]
pub trait Dome: Device + Send + Sync {
    /// The dome altitude (degrees, horizon zero and increasing positive to 90 zenith).
    #[http("altitude", method = Get)]
    async fn altitude(&self) -> ASCOMResult<f64> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Indicates whether the dome is in the home position.
    ///
    /// This is normally used following a FindHome()  operation. The value is reset with any azimuth slew operation that moves the dome away from the home position. AtHome may also become true durng normal slew operations, if the dome passes through the home position and the dome controller hardware is capable of detecting that; or at the end of a slew operation if the dome comes to rest at the home position.
    #[http("athome", method = Get)]
    async fn at_home(&self) -> ASCOMResult<bool> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// True if the dome is in the programmed park position.
    ///
    /// Set only following a Park() operation and reset with any slew operation.
    #[http("atpark", method = Get)]
    async fn at_park(&self) -> ASCOMResult<bool> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Returns the dome azimuth (degrees, North zero and increasing clockwise, i.e., 90 East, 180 South, 270 West).
    #[http("azimuth", method = Get)]
    async fn azimuth(&self) -> ASCOMResult<f64> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// True if the dome can move to the home position.
    #[http("canfindhome", method = Get)]
    async fn can_find_home(&self) -> ASCOMResult<bool> {
        Ok(false)
    }

    /// True if the dome is capable of programmed parking (Park() method).
    #[http("canpark", method = Get)]
    async fn can_park(&self) -> ASCOMResult<bool> {
        Ok(false)
    }

    /// True if driver is capable of setting the dome altitude.
    #[http("cansetaltitude", method = Get)]
    async fn can_set_altitude(&self) -> ASCOMResult<bool> {
        Ok(false)
    }

    /// True if driver is capable of setting the dome azimuth.
    #[http("cansetazimuth", method = Get)]
    async fn can_set_azimuth(&self) -> ASCOMResult<bool> {
        Ok(false)
    }

    /// True if driver is capable of setting the dome park position.
    #[http("cansetpark", method = Get)]
    async fn can_set_park(&self) -> ASCOMResult<bool> {
        Ok(false)
    }

    /// True if driver is capable of automatically operating shutter.
    #[http("cansetshutter", method = Get)]
    async fn can_set_shutter(&self) -> ASCOMResult<bool> {
        Ok(false)
    }

    /// True if driver is capable of slaving to a telescope.
    #[http("canslave", method = Get)]
    async fn can_slave(&self) -> ASCOMResult<bool> {
        Ok(false)
    }

    /// True if driver is capable of synchronizing the dome azimuth position using the SyncToAzimuth(Double) method.
    #[http("cansyncazimuth", method = Get)]
    async fn can_sync_azimuth(&self) -> ASCOMResult<bool> {
        Ok(false)
    }

    /// Returns the status of the dome shutter or roll-off roof.
    #[http("shutterstatus", method = Get)]
    async fn shutter_status(&self) -> ASCOMResult<ShutterState> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// True if the dome is slaved to the telescope in its hardware, else False.
    #[http("slaved", method = Get)]
    async fn slaved(&self) -> ASCOMResult<bool> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Sets the current subframe height.
    #[http("slaved", method = Put)]
    async fn set_slaved(&self, #[http("Slaved")] slaved: bool) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// True if any part of the dome is currently moving, False if all dome components are steady.
    #[http("slewing", method = Get)]
    async fn slewing(&self) -> ASCOMResult<bool> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Calling this method will immediately disable hardware slewing (Slaved will become False).
    #[http("abortslew", method = Put)]
    async fn abort_slew(&self) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Close the shutter or otherwise shield telescope from the sky.
    #[http("closeshutter", method = Put)]
    async fn close_shutter(&self) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// After Home position is established initializes Azimuth to the default value and sets the AtHome flag.
    #[http("findhome", method = Put)]
    async fn find_home(&self) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Open shutter or otherwise expose telescope to the sky.
    #[http("openshutter", method = Put)]
    async fn open_shutter(&self) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// After assuming programmed park position, sets AtPark flag.
    #[http("park", method = Put)]
    async fn park(&self) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Set the current azimuth, altitude position of dome to be the park position.
    #[http("setpark", method = Put)]
    async fn set_park(&self) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Slew the dome to the given altitude position.
    #[http("slewtoaltitude", method = Put)]
    async fn slew_to_altitude(&self, #[http("Altitude")] altitude: f64) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Slew the dome to the given azimuth position.
    #[http("slewtoazimuth", method = Put)]
    async fn slew_to_azimuth(&self, #[http("Azimuth")] azimuth: f64) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Synchronize the current position of the dome to the given azimuth.
    #[http("synctoazimuth", method = Put)]
    async fn sync_to_azimuth(&self, #[http("Azimuth")] azimuth: f64) -> ASCOMResult<()> {
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

/// Indicates the current state of the shutter or roof.
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
pub enum ShutterState {
    /// The shutter or roof is open.
    Open = 0,

    /// The shutter or roof is closed.
    Closed = 1,

    /// The shutter or roof is opening.
    Opening = 2,

    /// The shutter or roof is closing.
    Closing = 3,

    /// The shutter or roof has encountered a problem.
    Error = 4,
}
