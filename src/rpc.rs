use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct OpaqueResponse(serde_json::Map<String, serde_json::Value>);

impl OpaqueResponse {
    pub(crate) fn try_from<T: Serialize>(value: T) -> axum::response::Result<Self> {
        let json = serde_json::to_value(value).map_err(|err| {
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                err.to_string(),
            )
        })?;

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
                $specific_device: Vec<std::sync::Arc<tokio::sync::Mutex<dyn $specific_device>>>,
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

    (@trait $(#[doc = $doc:literal])* $trait_name:ident: $($parent:path),* {
        $(
            $(#[doc = $method_doc:literal])*
            #[http($method_path:literal $(, $params_ty:ty)?)]
            fn $method_name:ident(& $($mut_self:ident)* $(, $param:ident: $param_ty:ty)* $(,)?) $(-> $return_type:ty)?;
        )*
    } {
        $($extra_trait_body:tt)*
    }) => {
        #[allow(unused_variables)]
        $(#[doc = $doc])*
        #[async_trait::async_trait]
        pub trait $trait_name: $($parent+)* {
            $($extra_trait_body)*

            $(
                $(#[doc = $method_doc])*
                async fn $method_name(& $($mut_self)* $(, $param: $param_ty)*) -> $crate::ASCOMResult$(<$return_type>)? {
                    Err($crate::ASCOMError::NOT_IMPLEMENTED)
                }
            )*
        }

        impl dyn $trait_name {
            /// Private inherent method for handling actions.
            /// This method could live on the trait itself, but then it wouldn't be possible to make it private.
            async fn handle_action(device: &mut (impl ?Sized + $trait_name), is_mut: bool, action: &str, params: $crate::transaction::ASCOMParams) -> axum::response::Result<$crate::ASCOMResult<$crate::OpaqueResponse>> {
                use tracing::Instrument;

                match (is_mut, action) {
                    $((rpc!(@is_mut $($mut_self)*), $method_path) => async move {
                        $(
                            let params: $params_ty =
                                params.try_as()
                                .map_err(|err| {
                                    tracing::error!(%err, "Could not decode params");
                                    (axum::http::StatusCode::BAD_REQUEST, err.to_string())
                                })?;
                        )?
                        tracing::info!($($param = ?&params.$param,)* "Calling Alpaca handler");
                        Ok(match device.$method_name($(params.$param),*).await {
                            Ok(value) => Ok($crate::OpaqueResponse::try_from(value)?),
                            Err(err) => Err(err),
                        })
                    }.instrument(tracing::info_span!(concat!(stringify!($trait_name), "::", stringify!($method_name)))).await,)*
                    _ => return rpc!(@if_specific $trait_name {
                        <dyn Device>::handle_action(device, is_mut, action, params).await
                    } {
                        Err((axum::http::StatusCode::NOT_FOUND, "Unknown action").into())
                    })
                }
            }
        }
    };

    ($(
        $(#[doc = $doc:literal])*
        #[http($path:literal)]
        pub trait $trait_name:ident $trait_body:tt
    )*) => {
        rpc!(@storage $($trait_name)*);

        $(
            rpc!(@if_specific $trait_name {
                rpc!(@trait $(#[doc = $doc])* $trait_name: Device, Send, Sync $trait_body {
                    /// Register this device in the storage.
                    /// This method should not be overridden by implementors.
                    fn add_to(self, storage: &mut Devices) where Self: Sized + 'static {
                        storage.$trait_name.push(std::sync::Arc::new(tokio::sync::Mutex::new(self)));
                    }
                });
            } {
                rpc!(@trait $(#[doc = $doc])* $trait_name: std::fmt::Debug, Send, Sync $trait_body {
                    /// Unique ID of this device, ideally UUID.
                    async fn unique_id(&self) -> String;
                });
            });

            rpc!(@if_specific $trait_name {
                impl dyn $trait_name {
                    pub(crate) async fn get_in(storage: &Devices, device_number: usize) -> Option<tokio::sync::OwnedMutexGuard<dyn $trait_name>> {
                        Some(storage.$trait_name.get(device_number)?.clone().lock_owned().await)
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
            pub(crate) fn iter(&self) -> impl '_ + futures::Stream<Item = ConfiguredDevice> {
                async_stream::stream! {
                    $(
                        rpc!(@if_specific $trait_name {
                            for (device_number, device) in self.$trait_name.iter().enumerate() {
                                let device = device.lock().await;
                                let device = ConfiguredDevice {
                                    device_name: device.name().await.unwrap_or_default(),
                                    device_type: stringify!($trait_name).to_owned(),
                                    device_number,
                                    unique_id: device.unique_id().await.to_owned(),
                                };
                                tracing::debug!(?device, "Reporting device");
                                yield device;
                            }
                        });
                    )*
                }
            }

            pub(crate) async fn handle_action(&self, device_type: &str, device_number: usize, is_mut: bool, action: &str, params: $crate::transaction::ASCOMParams) -> axum::response::Result<$crate::ASCOMResult<$crate::OpaqueResponse>> {
                $(
                    rpc!(@if_specific $trait_name {
                        if device_type == $path {
                            let mut device = <dyn $trait_name>::get_in(self, device_number).await.ok_or((axum::http::StatusCode::NOT_FOUND, "Device not found"))?;
                            return <dyn $trait_name>::handle_action(&mut *device, is_mut, action, params).await;
                        }
                    });
                )*
                Err((axum::http::StatusCode::NOT_FOUND, "Unknown device type").into())
            }
        }
    };
}

pub(crate) use rpc;
