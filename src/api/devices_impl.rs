use crate::api::{Device, DeviceType, Devices};

pub(crate) trait RetrieavableDevice: 'static + Device /* where Self: Unsize<DynTrait> */ {
    const TYPE: DeviceType;

    fn get_storage(storage: &Devices) -> &[std::sync::Arc<Self>];
}

/// A trait for devices that can be registered in a `Devices` storage.
///
/// DynTrait is unused here, it's only necessary to cheat the type system
/// and allow "overlapping" blanket impls of RegistrableDevice for different
/// kinds of devices so that `devices.register(device)` "just works".
pub(crate) trait RegistrableDevice<DynTrait: ?Sized> {
    fn add_to(self, storage: &mut Devices);
}

// we use internal interfaces to get type inference magic to work with polymorphic device types
#[allow(private_bounds)]
impl Devices {
    /// Register a device in the storage.
    ///
    /// `device` can be an instance of any of the category traits (`Camera`, `Telescope`, etc.).
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
