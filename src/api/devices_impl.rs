use super::{ConfiguredDevice, Device, DeviceType, Devices, TypedDevice};
use serde::Serialize;
use std::fmt::{Debug, Display};

pub(crate) trait RetrieavableDevice: 'static + Device /* where Self: Unsize<DynTrait> */ {
    const TYPE: DeviceType;

    fn get_storage(storage: &Devices) -> &[std::sync::Arc<Self>];

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

// we use internal interfaces to get type inference magic to work with polymorphic device types
#[allow(private_bounds)]
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
    ) -> impl '_ + Iterator<Item = std::sync::Arc<DynTrait>> {
        DynTrait::get_storage(self)
            .iter()
            .map(std::sync::Arc::clone)
    }

    /// Retrieve a device by its category trait and an index within that category.
    ///
    /// Example: `devices.get::<dyn Camera>(0)` returns the first camera in the storage.
    pub fn get<DynTrait: ?Sized + RetrieavableDevice>(
        &self,
        device_number: usize,
    ) -> Option<&DynTrait> {
        DynTrait::get_storage(self)
            .get(device_number)
            .map(std::sync::Arc::as_ref)
    }

    #[cfg(feature = "server")]
    pub(crate) fn get_for_server<DynTrait: ?Sized + RetrieavableDevice>(
        &self,
        device_number: usize,
    ) -> crate::server::Result<&DynTrait> {
        self.get::<DynTrait>(device_number)
            .ok_or(crate::server::Error::UnknownDeviceIndex {
                ty: DynTrait::TYPE,
                index: device_number,
            })
    }
}

pub(crate) struct FallibleDeviceType(pub(crate) Result<DeviceType, String>);

impl Debug for FallibleDeviceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            Ok(ty) => Debug::fmt(ty, f),
            Err(ty) => write!(f, "Unsupported({ty})"),
        }
    }
}

impl Display for DeviceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Debug for DeviceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl Serialize for DeviceType {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.as_str().serialize(serializer)
    }
}

#[derive(PartialEq, Eq, Clone, Copy)]
pub(crate) struct DevicePath(pub(crate) DeviceType);

impl Display for DevicePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Debug for DevicePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
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
