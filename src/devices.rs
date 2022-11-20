use crate::api::Device;
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};

type DevicesStorage =
    HashMap<(&'static str, usize), Box<Mutex<dyn Device + Send + Sync + 'static>>>;

impl fmt::Debug for dyn Device + Send + Sync {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct(self.ty())
            .field("name", &self.name())
            .field("description", &self.description())
            .finish()
    }
}

#[derive(Debug, Default)]
pub struct DevicesBuilder {
    devices: DevicesStorage,
    counters: HashMap<&'static str, usize>,
}

impl DevicesBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with<T: Device + Send + Sync + 'static>(mut self, device: T) -> Self {
        let index_ref = self.counters.entry(device.ty()).or_insert(0);
        let index = *index_ref;
        assert!(self
            .devices
            .insert((device.ty(), index), Box::new(Mutex::new(device)))
            .is_none());
        *index_ref += 1;
        self
    }

    pub fn finish(self) -> Devices {
        Devices {
            devices: Arc::new(self.devices),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Devices {
    devices: Arc<DevicesStorage>,
}

impl Devices {
    pub fn get<'inputs>(
        &'inputs self,
        device_type: &'inputs str,
        device_number: usize,
    ) -> Option<&'inputs Mutex<dyn Device + Send + Sync + 'static>> {
        match self.devices.get(&(device_type, device_number)) {
            Some(device) => Some(device),
            None => {
                tracing::error!(device_type, device_number, "Device not found");
                None
            }
        }
    }
}
