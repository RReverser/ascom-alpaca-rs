use crate::{ASCOMError, ASCOMErrorCode, ASCOMResult};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct OpaqueResponse(serde_json::Map<String, serde_json::Value>);

impl OpaqueResponse {
    pub(crate) fn try_from<T: Serialize>(value: T) -> ASCOMResult<Self> {
        let json = serde_json::to_value(value)
            .map_err(|err| ASCOMError::new(ASCOMErrorCode::INVALID_VALUE, err.to_string()))?;

        Ok(Self(match json {
            serde_json::Value::Object(map) => map,
            serde_json::Value::Null => serde_json::Map::new(),
            value => {
                // Wrap into IntResponse / BoolResponse / ..., aka {"value": ...}
                std::iter::once(("Value".to_owned(), value)).collect()
            }
        }))
    }
}

macro_rules! rpc {
    (@if_specific Device $then:tt $({ $($else:tt)* })?) => {
        $($($else)*)?
    };

    (@if_specific $trait_name:ident { $($then:tt)* } $($else:tt)?) => {
        $($then)*
    };

    (@is_mut mut self) => (true);

    (@is_mut self) => (false);

    (@storage $device:ident $($specific_device:ident)*) => {
        #[allow(non_snake_case)]
        pub struct Devices {
            $(
                $specific_device: Vec<Box<std::sync::Mutex<dyn $specific_device + Send + Sync>>>,
            )*
        }

        impl Default for Devices {
            fn default() -> Self {
                Self {
                    $(
                        $specific_device: Vec::new(),
                    )*
                }
            }
        }

        impl std::fmt::Debug for Devices {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let mut f = f.debug_struct("Devices");
                $(
                    if !self.$specific_device.is_empty() {
                        let _ = f.field(stringify!($specific_device), &self.$specific_device);
                    }
                )*
                f.finish()
            }
        }
    };

    ($(
        $(#[doc = $doc:literal])*
        #[http($path:literal)]
        pub trait $trait_name:ident $(: $parent_trait_name:ident)? {
            $(
                $(#[doc = $method_doc:literal])*
                #[http($method_path:literal $(, $params_ty:ty)?)]
                fn $method_name:ident(& $($mut_self:ident)* $(, $param:ident: $param_ty:ty)* $(,)?) $(-> $return_type:ty)?;
            )*
        }
    )*) => {
        rpc!(@storage $($trait_name)*);

        $(
            impl std::fmt::Debug for dyn $trait_name + Send + Sync {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    f.debug_struct(stringify!($trait_name))
                    .field("name", &self.name())
                    .field("description", &self.description())
                    .field("driver_info", &self.driver_info())
                    .field("driver_version", &self.driver_version())
                    .finish()
                }
            }
        )*

        $(
            #[allow(unused_variables)]
            $(#[doc = $doc])*
            pub trait $trait_name $(: $parent_trait_name)? {
                rpc!(@if_specific $trait_name {
                    /// Register this device in the storage.
                    /// This method should not be overridden by implementors.
                    fn add_to(self, storage: &mut Devices) where Self: Sized + Send + Sync + 'static {
                        storage.$trait_name.push(Box::new(std::sync::Mutex::new(self)));
                    }
                } {
                    /// Unique ID of this device, ideally a UUID.
                    fn unique_id(&self) -> &str;
                });

                $(
                    $(#[doc = $method_doc])*
                    fn $method_name(& $($mut_self)* $(, $param: $param_ty)*) -> $crate::ASCOMResult$(<$return_type>)? {
                        Err($crate::ASCOMError::NOT_IMPLEMENTED)
                    }
                )*
            }

            impl dyn $trait_name {
                /// Private inherent method for handling actions.
                /// This method could live on the trait itself, but then it wouldn't be possible to make it private.
                fn handle_action(device: &mut (impl ?Sized + $trait_name), is_mut: bool, action: &str, params: $crate::transaction::ASCOMParams) -> $crate::ASCOMResult<$crate::OpaqueResponse> {
                    match (is_mut, action) {
                        $((rpc!(@is_mut $($mut_self)*), $method_path) => {
                            let span = tracing::info_span!(concat!(stringify!($trait_name), "::", stringify!($method_name)));
                            let _enter = span.enter();

                            $(
                                let params: $params_ty =
                                    params.try_as()
                                    .map_err(|err| {
                                        tracing::error!(raw_params = ?params, ?err, "Could not decode params");
                                        $crate::ASCOMError::new($crate::ASCOMErrorCode::INVALID_VALUE, err.to_string())
                                    })?;
                            )?
                            tracing::info!($($param = ?&params.$param,)* "Calling Alpaca handler");
                            let result = device.$method_name($(params.$param),*)?;
                            tracing::debug!(?result, "Alpaca handler returned");
                            $crate::OpaqueResponse::try_from(result)
                        })*
                        _ => {
                            rpc!(@if_specific $trait_name {
                                <dyn Device>::handle_action(device, is_mut, action, params)
                            } {
                                Err($crate::ASCOMError::NOT_IMPLEMENTED)
                            })
                        }
                    }
                }
            }

            rpc!(@if_specific $trait_name {
                impl dyn $trait_name {
                    pub fn with<T>(storage: &Devices, device_number: usize, f: impl FnOnce(&mut dyn $trait_name) -> T) -> Result<T, (axum::http::StatusCode, &'static str)> {
                        let mut device =
                            storage.$trait_name.get(device_number)
                            .ok_or((axum::http::StatusCode::NOT_FOUND, "Device not found"))?
                            .lock()
                            .map_err(|_err| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "This device can't be accessed anymore due to a previous fatal error"))?;

                        Ok(f(&mut *device))
                    }
                }
            });
        )*

        #[derive(Debug, Serialize, Deserialize)]
        #[serde(rename_all = "PascalCase")]
        pub struct ConfiguredDevice {
            pub device_name: String,
            pub device_type: String,
            pub device_number: usize,
            #[serde(rename = "UniqueID")]
            pub unique_id: String,
        }

        impl Devices {
            pub fn iter(&self) -> impl '_ + Iterator<Item = ConfiguredDevice> + Clone {
                let iter = std::iter::empty();
                $(
                    rpc!(@if_specific $trait_name {
                        let iter = iter.chain(self.$trait_name.iter().enumerate().filter_map(|(device_number, device)| {
                            let device = device.lock().ok()?;
                            Some(ConfiguredDevice {
                                device_name: device.name().unwrap_or_default(),
                                device_type: stringify!($trait_name).into(),
                                device_number,
                                unique_id: device.unique_id().to_owned(),
                            })
                        }));
                    });
                )*
                iter
            }

            pub fn handle_action(&self, device_type: &str, device_number: usize, is_mut: bool, action: &str, params: $crate::transaction::ASCOMParams) -> Result<$crate::ASCOMResult<$crate::OpaqueResponse>, (axum::http::StatusCode, &'static str)> {
                $(
                    rpc!(@if_specific $trait_name {
                        if device_type == $path {
                            return <dyn $trait_name>::with(self, device_number, |device| {
                                <dyn $trait_name>::handle_action(device, is_mut, action, params)
                            });
                        }
                    });
                )*
                Err((axum::http::StatusCode::NOT_FOUND, "Unknown device type"))
            }
        }
    };
}

pub(crate) use rpc;
