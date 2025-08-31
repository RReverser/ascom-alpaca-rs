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
    #[http("getswitch", method = Get /* TODO: , device_state = GetSwitch */)]
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
    #[http("getswitchvalue", method = Get /* TODO: , device_state = GetSwitchValue */)]
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
    #[http("statechangecomplete", method = Get /* TODO:, device_state = StateChangeComplete */)]
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

/// An object representing operational properties of a specific device connected to the switch.
#[derive(Default, Debug, Clone, Copy)]
pub struct SwitchDeviceState {
    /// Result of [`Switch::get_switch`].
    pub get_switch: Option<bool>,
    /// Result of [`Switch::get_switch_value`].
    pub get_switch_value: Option<f64>,
    /// Result of [`Switch::state_change_complete`].
    pub state_change_complete: Option<bool>,
}

impl SwitchDeviceState {
    async fn gather(switch: &(impl ?Sized + Switch), id: i32) -> Self {
        Self {
            get_switch: switch.get_switch(id).await.ok(),
            get_switch_value: switch.get_switch_value(id).await.ok(),
            state_change_complete: switch.state_change_complete(id).await.ok(),
        }
    }
}

/// An object representing all operational properties of the device.
#[derive(Default, Debug, Clone)]
pub struct DeviceState {
    /// States of individual switch devices, indexed by their ID.
    pub switch_devices: Vec<SwitchDeviceState>,
}

impl DeviceState {
    async fn new(switch: &(impl ?Sized + Switch)) -> Self {
        Self {
            switch_devices: match switch.max_switch().await {
                Ok(n) => {
                    futures::future::join_all(
                        (0_i32..n).map(|id| SwitchDeviceState::gather(switch, id)),
                    )
                    .await
                }
                Err(err) => {
                    tracing::error!(%err, "Failed to get max switch");
                    Vec::new()
                }
            },
        }
    }
}

#[cfg(feature = "server")]
impl serde::Serialize for DeviceState {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;

        let mut map = serializer.serialize_map(None)?;
        for (i, device) in self.switch_devices.iter().enumerate() {
            if let Some(value) = &device.get_switch {
                map.serialize_entry(&format!("GetSwitch{i}"), value)?;
            }
            if let Some(value) = &device.get_switch_value {
                map.serialize_entry(&format!("GetSwitchValue{i}"), value)?;
            }
            if let Some(value) = &device.state_change_complete {
                map.serialize_entry(&format!("StateChangeComplete{i}"), value)?;
            }
        }
        map.end()
    }
}

#[cfg(feature = "client")]
impl<'de> serde::Deserialize<'de> for DeviceState {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de;

        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = DeviceState;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("device state object")
            }

            fn visit_map<A: de::MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut state = DeviceState::default();

                while let Some(name) = map.next_key::<&'de str>()? {
                    // This is pretty complicated because we want to transform shape like `{Name: "GetSwitch2", Value}` into `switch_devices[2].get_switch = Value`.
                    let index_start = name.find(|c: char| c.is_ascii_digit()).ok_or_else(|| {
                        de::Error::custom(format!("could not find switch device index in {name:?}"))
                    })?;
                    let (name, index) = name.split_at(index_start);
                    let index = index.parse::<usize>().map_err(|err| {
                        de::Error::custom(format_args!(
                            "could not parse switch device index {index:?}: {err}"
                        ))
                    })?;
                    // Auto-extend the vec to accommodate the new index. We don't have access to total number of devices here without another async call,
                    // so we have to make guesses based on the returned data.
                    if index >= state.switch_devices.len() {
                        state
                            .switch_devices
                            .resize_with(index + 1, SwitchDeviceState::default);
                    }
                    let switch_device = &mut state.switch_devices[index];
                    match name {
                        "GetSwitch" => {
                            switch_device.get_switch = Some(map.next_value()?);
                        }
                        "GetSwitchValue" => {
                            switch_device.get_switch_value = Some(map.next_value()?);
                        }
                        "StateChangeComplete" => {
                            switch_device.state_change_complete = Some(map.next_value()?);
                        }
                        other => {
                            return Err(de::Error::unknown_field(
                                other,
                                &["GetSwitch", "GetSwitchValue", "StateChangeComplete"],
                            ));
                        }
                    }
                }

                Ok(state)
            }
        }

        deserializer.deserialize_map(Visitor)
    }
}
