use crate::api::Device;
use crate::{ASCOMError, ASCOMResult, OpaqueResponse};
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
    pub fn handle_action(
        &self,
        is_mut: bool,
        device_type: &str,
        device_number: usize,
        action: &str,
        params: &str,
    ) -> ASCOMResult<OpaqueResponse> {
        self.devices
            .get(&(device_type, device_number))
            .ok_or(ASCOMError::NOT_CONNECTED)?
            .lock()
            .expect("Device lock is poisoned")
            .handle_action(is_mut, action, params)
    }
}
