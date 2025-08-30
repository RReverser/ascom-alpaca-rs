/*!
ASCOM Alpaca Device API v1

The Alpaca API uses RESTful techniques and TCP/IP to enable ASCOM applications and devices to communicate across modern network environments.

## Interface Versions
These interface definitions include the updates introduced in **ASCOM Platform 7**.

## Interface Behaviour
The ASCOM Interface behavioural requirements for Alpaca drivers are the same as for COM based drivers and are documented in the <a href="https://ascom-standards.org/Help/Developer/html/N_ASCOM_DeviceInterface.htm">API Interface Definitions</a> e.g. the <a href="https://ascom-standards.org/Help/Developer/html/M_ASCOM_DeviceInterface_ITelescopeV3_SlewToCoordinates.htm">Telescope.SlewToCoordinates</a> method.       This document focuses on how to use the ASCOM Interface standards in their RESTful Alpaca form.
*/

#![expect(clippy::doc_markdown)]

mod server_info;
pub use server_info::*;

#[cfg(any(feature = "camera", feature = "telescope"))]
mod camera_telescope_shared;

mod time_repr;

use std::fmt::Debug;
use std::sync::Arc;

#[macro_use]
mod macros;

/// Types related to the general [`Device`] trait.
pub mod device;
pub use device::Device;

/// A helper alias for the common type of futures returned by device traits.
///
/// You normally don't need to use it as long as you use `#[async_trait]` - it's mostly here for documentation purposes.
pub type ASCOMResultFuture<'async_trait, T> =
    futures::future::BoxFuture<'async_trait, crate::ASCOMResult<T>>;

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
    ///
    /// Note that you don't need to provide the generic type parameter - it's here only for type
    /// inference purposes to allow "overloads" that automatically register device into the correct
    /// storage.
    #[tracing::instrument(level = "debug", skip(self))]
    pub fn register<DynTrait: ?Sized>(&mut self, device: impl RegistrableDevice<DynTrait>) {
        device.add_to(self);
    }

    /// Iterate over all devices of a given type.
    pub fn iter<DynTrait: ?Sized + RetrieavableDevice>(
        &self,
    ) -> impl ExactSizeIterator<Item = Arc<DynTrait>> {
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
    ) -> crate::server::Result<Arc<DynTrait>> {
        DynTrait::get_storage(self)
            .get(device_number)
            .map(Arc::clone)
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

#[cfg(feature = "server")]
#[derive(serde::Serialize)]
#[serde(rename_all = "PascalCase")]
struct DeviceStateItem<T> {
    name: std::borrow::Cow<'static, str>,
    value: T,
}

#[cfg(feature = "server")]
impl<T: serde::Serialize> DeviceStateItem<T> {
    fn serialize<S: serde::ser::SerializeSeq>(
        name: impl Into<std::borrow::Cow<'static, str>>,
        value: Option<T>,
        seq: &mut S,
    ) -> Result<(), S::Error> {
        value.map_or_else(
            || Ok(()),
            |value| {
                serde::ser::SerializeSeq::serialize_element(
                    seq,
                    &Self {
                        name: name.into(),
                        value,
                    },
                )
            },
        )
    }
}

#[cfg(feature = "server")]
impl DeviceStateItem<time_repr::TimeRepr<time_repr::Iso8601>> {
    #[expect(clippy::ref_option)]
    fn serialize_timestamp<S: serde::ser::SerializeSeq>(
        timestamp: &Option<std::time::SystemTime>,
        seq: &mut S,
    ) -> Result<(), S::Error> {
        Self::serialize("TimeStamp", timestamp.map(From::from), seq)
    }
}
