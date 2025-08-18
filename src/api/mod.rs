/*!
ASCOM Alpaca Device API v1

The Alpaca API uses RESTful techniques and TCP/IP to enable ASCOM applications and devices to communicate across modern network environments.

## Interface Versions
These interface definitions include the updates introduced in **ASCOM Platform 7**.

## Interface Behaviour
The ASCOM Interface behavioural requirements for Alpaca drivers are the same as for COM based drivers and are documented in the <a href="https://ascom-standards.org/Help/Developer/html/N_ASCOM_DeviceInterface.htm">API Interface Definitions</a> e.g. the <a href="https://ascom-standards.org/Help/Developer/html/M_ASCOM_DeviceInterface_ITelescopeV3_SlewToCoordinates.htm">Telescope.SlewToCoordinates</a> method.       This document focuses on how to use the ASCOM Interface standards in their RESTful Alpaca form.
## Alpaca URLs, Case Sensitivity, Parameters and Returned values
**Alpaca Device API URLs** are of the form **http(s)://host:port/path** where path comprises **"/api/v1/"** followed by one of the method names below. e.g. for an Alpaca interface running on port 7843 of a device with IP address 192.168.1.89:
* A telescope "Interface Version" method URL would be **http://192.168.1.89:7843/api/v1/telescope/0/interfaceversion**

* A first focuser "Position" method URL would be  **http://192.168.1.89:7843/api/v1/focuser/0/position**

* A second focuser "StepSize" method URL would be  **http://192.168.1.89:7843/api/v1/focuser/1/stepsize**
* A rotator "Halt" method URL would be  **http://192.168.1.89:7843/api/v1/rotator/0/halt**


URLs are case sensitive and all elements must be in lower case. This means that both the device type and command name must always be in lower case. Parameter names are not case sensitive, so clients and drivers should be prepared for parameter names to be supplied and returned with any casing. Parameter values can be in mixed case as required.

For GET operations, parameters should be placed in the URL query string and for PUT operations they should be placed in the body of the message.

Responses, as described below, are returned in JSON format and always include a common set of values including the client's transaction number,  the server's transaction number together with any error message and error number.
If the transaction completes successfully, the ErrorMessage field will be an empty string and the ErrorNumber field will be zero.

## HTTP Status Codes and ASCOM Error codes
The returned HTTP status code gives a high level view of whether the device understood the request and whether it attempted to process it.

Under most circumstances the returned status will be `200`, indicating that the request was correctly formatted and that it was passed to the device's handler to execute. A `200` status does not necessarily mean that the operation completed as expected, without error, and you must always check the ErrorMessage and ErrorNumber fields to confirm whether the returned result is valid. The `200` status simply means that the transaction was successfully managed by the device's transaction management layer.

An HTTP status code of `400` indicates that the device could not interpret the request e.g. an invalid device number or misspelt device type was supplied. Check the body of the response for a text error message.

An HTTP status code of `500` indicates an unexpected error within the device from which it could not recover. Check the body of the response for a text error message.
*/

#![expect(clippy::doc_markdown)]

mod server_info;
pub use server_info::*;

#[cfg(any(feature = "camera", feature = "telescope"))]
mod camera_telescope_shared;

use std::fmt::Debug;
use std::sync::Arc;

#[macro_use]
mod macros;

/// Types related to the general [`Device`] trait.
pub mod device;
pub use device::Device;

rpc_mod! {
    #[cfg(feature = "camera")]
    Camera = "camera",

    #[cfg(feature = "cover_calibrator")]
    CoverCalibrator = "covercalibrator",

    #[cfg(feature = "dome")]
    Dome = "dome",

    #[cfg(feature = "filter_wheel")]
    FilterWheel = "filterwheel",

    #[cfg(feature = "focuser")]
    Focuser = "focuser",

    #[cfg(feature = "observing_conditions")]
    ObservingConditions = "observingconditions",

    #[cfg(feature = "rotator")]
    Rotator = "rotator",

    #[cfg(feature = "safety_monitor")]
    SafetyMonitor = "safetymonitor",

    #[cfg(feature = "switch")]
    Switch = "switch",

    #[cfg(feature = "telescope")]
    Telescope = "telescope",
}

pub(crate) trait RetrieavableDevice: 'static + Device {
    #[allow(unused)]
    const TYPE: DeviceType;

    fn get_storage(storage: &Devices) -> &[Arc<Self>];

    #[cfg(feature = "server")]
    fn to_configured_device(&self, as_number: usize) -> ConfiguredDevice<DeviceType> {
        ConfiguredDevice {
            name: self.static_name().to_owned(),
            ty: Self::TYPE,
            number: as_number,
            unique_id: self.unique_id().to_owned(),
        }
    }
}

/// A trait for devices that can be registered in a `Devices` storage.
///
/// DynTrait is unused here, it's only necessary to cheat the type system
/// and allow "overlapping" blanket impls of RegistrableDevice for different
/// kinds of devices so that `devices.register(device)` "just works".
pub(crate) trait RegistrableDevice<DynTrait: ?Sized>: Debug {
    fn add_to(self, storage: &mut Devices);
}

impl Default for Devices {
    fn default() -> Self {
        // Invoke the inherent const implementation.
        Self::default()
    }
}

// we use internal interfaces to get type inference magic to work with polymorphic device types
#[expect(private_bounds)]
impl Devices {
    /// Register a device in the storage.
    ///
    /// `device` can be an instance of any of the category traits (`Camera`, `Telescope`, etc.).
    #[tracing::instrument(level = "debug", skip(self))]
    pub fn register<DynTrait: ?Sized>(&mut self, device: impl RegistrableDevice<DynTrait>) {
        device.add_to(self);
    }

    /// Iterate over all devices of a given type.
    pub fn iter<DynTrait: ?Sized + RetrieavableDevice>(
        &self,
    ) -> impl '_ + ExactSizeIterator<Item = Arc<DynTrait>> {
        DynTrait::get_storage(self).iter().map(Arc::clone)
    }

    /// Retrieve a device by its category trait and an index within that category.
    ///
    /// Example: `devices.get::<dyn Camera>(0)` returns the first camera in the storage.
    pub fn get<DynTrait: ?Sized + RetrieavableDevice>(
        &self,
        device_number: usize,
    ) -> Option<Arc<DynTrait>> {
        DynTrait::get_storage(self).get(device_number).cloned()
    }

    #[cfg(feature = "server")]
    pub(crate) fn get_for_server<DynTrait: ?Sized + RetrieavableDevice>(
        &self,
        device_number: usize,
    ) -> crate::server::Result<&DynTrait> {
        DynTrait::get_storage(self)
            .get(device_number)
            .map(Arc::as_ref)
            .ok_or(crate::server::Error::UnknownDeviceNumber {
                ty: DynTrait::TYPE,
                device_number,
            })
    }
}

impl Extend<TypedDevice> for Devices {
    fn extend<T: IntoIterator<Item = TypedDevice>>(&mut self, iter: T) {
        for client in iter {
            self.register(client);
        }
    }
}

impl FromIterator<TypedDevice> for Devices {
    fn from_iter<T: IntoIterator<Item = TypedDevice>>(iter: T) -> Self {
        let mut devices = Self::default();
        devices.extend(iter);
        devices
    }
}
