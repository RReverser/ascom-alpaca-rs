use crate::api::{Device, DeviceType, Devices};

// NOTE: this is a Voldemort trait, not meant to be really public.
pub trait RetrieavableDevice: 'static + Device /* where Self: Unsize<DynTrait> */ {
    const TYPE: DeviceType;

    fn get_storage(storage: &Devices) -> &[std::sync::Arc<Self>];
}

// NOTE: this is a Voldemort trait, not meant to be really public.
pub trait RegistrableDevice<DynTrait: ?Sized + RetrieavableDevice> {
    fn add_to(self, storage: &mut Devices);
}

impl Devices {
    pub fn register<DynTrait: ?Sized + RetrieavableDevice>(
        &mut self,
        device: impl RegistrableDevice<DynTrait>,
    ) {
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
        self.get(device_number).ok_or_else(|| {
            crate::server::Error::NotFound(anyhow::anyhow!(
                "Device {}#{} not found",
                DynTrait::TYPE,
                device_number
            ))
        })
    }

    pub fn iter<DynTrait: ?Sized + RetrieavableDevice>(
        &self,
    ) -> impl '_ + Iterator<Item = &DynTrait> {
        DynTrait::get_storage(self)
            .iter()
            .map(std::sync::Arc::as_ref)
    }
}
