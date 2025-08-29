use super::Device;
use crate::{ASCOMError, ASCOMResult};
use macro_rules_attribute::apply;

/// Switch Specific Methods.
#[apply(rpc_trait)]
pub trait Switch: Device + Send + Sync {
    /// Returns the number of switch devices managed by this driver.
    ///
    /// Devices are numbered from 0 to MaxSwitch - 1.
    #[http("maxswitch", method = Get)]
    async fn max_switch(&self) -> ASCOMResult<i32>;

    /// This endpoint must be implemented and indicates whether the given switch can operate asynchronously.
    ///
    /// _ISwitchV3 and later._
    #[http("canasync", method = Get)]
    async fn can_async(&self, #[http("Id")] id: i32) -> ASCOMResult<bool> {
        Ok(false)
    }

    /// Reports if the specified switch device can be written to, default true.
    ///
    /// This is false if the device cannot be written to, for example a limit switch or a sensor.  Devices are numbered from 0 to MaxSwitch - 1.
    #[http("canwrite", method = Get)]
    async fn can_write(&self, #[http("Id")] id: i32) -> ASCOMResult<bool> {
        Ok(false)
    }

    /// Return the state of switch device id as a boolean.  Devices are numbered from 0 to MaxSwitch - 1.
    #[http("getswitch", method = Get)]
    async fn get_switch(&self, #[http("Id")] id: i32) -> ASCOMResult<bool>;

    /// Gets the description of the specified switch device.
    ///
    /// This is to allow a fuller description of the device to be returned, for example for a tool tip. Devices are numbered from 0 to MaxSwitch - 1.
    #[http("getswitchdescription", method = Get)]
    async fn get_switch_description(&self, #[http("Id")] id: i32) -> ASCOMResult<String>;

    /// Gets the name of the specified switch device.
    ///
    /// Devices are numbered from 0 to MaxSwitch - 1.
    #[http("getswitchname", method = Get)]
    async fn get_switch_name(&self, #[http("Id")] id: i32) -> ASCOMResult<String>;

    /// Gets the value of the specified switch device as a double.
    ///
    /// Devices are numbered from 0 to MaxSwitch - 1, The value of this switch is expected to be between MinSwitchValue and MaxSwitchValue.
    #[http("getswitchvalue", method = Get)]
    async fn get_switch_value(&self, #[http("Id")] id: i32) -> ASCOMResult<f64>;

    /// Gets the minimum value of the specified switch device as a double.
    ///
    /// Devices are numbered from 0 to MaxSwitch - 1.
    #[http("minswitchvalue", method = Get)]
    async fn min_switch_value(&self, #[http("Id")] id: i32) -> ASCOMResult<f64>;

    /// Gets the maximum value of the specified switch device as a double.
    ///
    /// Devices are numbered from 0 to MaxSwitch - 1.
    #[http("maxswitchvalue", method = Get)]
    async fn max_switch_value(&self, #[http("Id")] id: i32) -> ASCOMResult<f64>;

    /// This is an asynchronous method that must return as soon as the state change operation has been successfully started,  with StateChangeComplete(Int16) for the given switch Id = False.  After the state change has completed StateChangeComplete(Int16) becomes True.
    ///
    /// _ISwitchV3 and later._
    #[http("setasync", method = Put)]
    async fn set_async(
        &self,
        #[http("Id")] id: i32,
        #[http("State")] state: bool,
    ) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// This is an asynchronous method that must return as soon as the state change operation has been successfully started,  with StateChangeComplete(Int16) for the given switch Id = False.  After the state change has completed StateChangeComplete(Int16) becomes True.
    ///
    /// _ISwitchV3 and later._
    #[http("setasyncvalue", method = Put)]
    async fn set_async_value(
        &self,

        #[http("Id")] id: i32,

        #[http("Value")] value: f64,
    ) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Sets a switch controller device to the specified state, true or false.
    #[http("setswitch", method = Put)]
    async fn set_switch(
        &self,
        #[http("Id")] id: i32,
        #[http("State")] state: bool,
    ) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Sets a switch device name to the specified value.
    #[http("setswitchname", method = Put)]
    async fn set_switch_name(
        &self,

        #[http("Id")] id: i32,

        #[http("Name")] name: String,
    ) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// Sets a switch device value to the specified value.
    #[http("setswitchvalue", method = Put)]
    async fn set_switch_value(
        &self,

        #[http("Id")] id: i32,

        #[http("Value")] value: f64,
    ) -> ASCOMResult<()> {
        Err(ASCOMError::NOT_IMPLEMENTED)
    }

    /// True if the state of the specified switch is changing, otherwise false.
    ///
    /// _ISwitchV3 and later._
    #[http("statechangecomplete", method = Get)]
    async fn state_change_complete(&self, #[http("Id")] id: i32) -> ASCOMResult<bool>;

    /// Returns the step size that this device supports (the difference between successive values of the device).
    ///
    /// Devices are numbered from 0 to MaxSwitch - 1.
    #[http("switchstep", method = Get)]
    async fn switch_step(&self, #[http("Id")] id: i32) -> ASCOMResult<f64>;

    /// This method returns the version of the ASCOM device interface contract to which this device complies.
    ///
    /// Only one interface version is current at a moment in time and all new devices should be built to the latest interface version. Applications can choose which device interface versions they support and it is in their interest to support  previous versions as well as the current version to ensure thay can use the largest number of devices.
    #[http("interfaceversion", method = Get)]
    async fn interface_version(&self) -> ASCOMResult<i32> {
        Ok(3_i32)
    }
}
