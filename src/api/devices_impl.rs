use crate::api::{Device, DeviceType, Devices};

// NOTE: this is a Voldemort trait, not meant to be really public.
pub trait RetrieavableDevice: 'static + Device /* where Self: Unsize<DynTrait> */ {
    const TYPE: DeviceType;

    fn get_storage(storage: &Devices) -> &[std::sync::Arc<Self>];
}

/// A trait for devices that can be registered in a `Devices` storage.
///
/// DynTrait is unused here, it's only necessary to cheat the type system
/// and allow "overlapping" blanket impls of RegistrableDevice for different
/// kinds of devices so that `devices.register(device)` "just works".
///
/// NOTE: this is a Voldemort trait, not meant to be really public.
pub trait RegistrableDevice<DynTrait: ?Sized> {
    fn add_to(self, storage: &mut Devices);
}

impl Devices {
    pub fn register<DynTrait: ?Sized>(&mut self, device: impl RegistrableDevice<DynTrait>) {
        device.add_to(self);
    }

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
    ) -> Result<&DynTrait, crate::server::Error> {
        self.get::<DynTrait>(device_number).ok_or_else(|| {
            crate::server::Error::NotFound(anyhow::anyhow!(
                "Device {}#{} not found",
                DynTrait::TYPE,
                device_number
            ))
        })
    }
}
