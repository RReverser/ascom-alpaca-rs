macro_rules! rpc {
    (@if_specific Device $then:tt $({ $($else:tt)* })?) => {
        $($($else)*)?
    };

    (@if_specific $trait_name:ident { $($then:tt)* } $($else:tt)?) => {
        $($then)*
    };

    (@is_mut mut $self:ident) => (true);

    (@is_mut $self:ident) => (false);

    (@get_self mut $self:ident) => ($self);

    (@get_self $self:ident) => ($self);

    (@storage $device:ident $($specific_device:ident)*) => {
        #[allow(non_snake_case)]
        #[derive(Default)]
        pub struct Devices {
            $(
                $specific_device: Vec<std::sync::Arc<tokio::sync::Mutex<dyn $specific_device>>>,
            )*
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

    (@trait $(#[doc = $doc:literal])* #[http($path:literal)] $trait_name:ident: $($parent:path),* {
        $(
            $(#[doc = $method_doc:literal])*
            #[http($method_path:literal)]
            fn $method_name:ident(& $($mut_self:ident)* $(, #[http($param_query:literal)] $param:ident: $param_ty:ty)* $(,)?) $(-> $return_type:ty)?;
        )*
    } {
        $($extra_trait_body:item)*
    } {
        $($extra_impl_body:item)*
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
            async fn handle_action(device: &mut (impl ?Sized + $trait_name), is_mut: bool, action: &str, #[allow(unused_mut)] mut params: $crate::params::OpaqueParams) -> axum::response::Result<$crate::ASCOMResult<$crate::response::OpaqueResponse>> {
                match (is_mut, action) {
                    $(
                        (rpc!(@is_mut $($mut_self)*), $method_path) => {
                            $(
                                let $param = params.extract($param_query).map_err(|err| (axum::http::StatusCode::BAD_REQUEST, format!("{err:#}")))?;
                            )*
                            params.finish_extraction();
                            Ok(device.$method_name($($param),*).await.map($crate::response::OpaqueResponse::new))
                        },
                    )*
                    _ => rpc!(@if_specific $trait_name {
                        <dyn Device>::handle_action(device, is_mut, action, params).await
                    } {
                        Err((axum::http::StatusCode::NOT_FOUND, "Unknown action").into())
                    })
                }
            }
        }

        #[async_trait::async_trait]
        impl $trait_name for $crate::client::Sender {
            $($extra_impl_body)*

            $(
                async fn $method_name(& $($mut_self)* $(, $param: $param_ty)*) -> $crate::ASCOMResult$(<$return_type>)? {
                    #[allow(unused_mut)]
                    let mut opaque_params = $crate::params::OpaqueParams::default();
                    $(
                        opaque_params.insert($param_query, $param);
                    )*
                    #[allow(unused_variables)]
                    let opaque_response = rpc!(@get_self $($mut_self)*).exec_action(rpc!(@is_mut $($mut_self)*), $method_path, opaque_params).await?;
                    Ok({
                        $(
                            // TODO: handle $.Value
                            opaque_response.try_as::<$return_type>()
                            .map_err(|err| $crate::ASCOMError::new($crate::ASCOMErrorCode::UNSPECIFIED, err.to_string()))?
                        )?
                    })
                }
            )*
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
                rpc!(@trait $(#[doc = $doc])* #[http($path)] $trait_name: Device, Send, Sync $trait_body {
                    /// Register this device in the storage.
                    /// This method should not be overridden by implementors.
                    fn add_to(self, storage: &mut Devices) where Self: Sized + 'static {
                        storage.$trait_name.push(std::sync::Arc::new(tokio::sync::Mutex::new(self)));
                    }
                } {});
            } {
                rpc!(@trait $(#[doc = $doc])* #[http($path)] $trait_name: std::fmt::Debug, Send, Sync $trait_body {
                    /// Unique ID of this device, ideally UUID.
                    fn unique_id(&self) -> &str;
                } {
                    fn unique_id(&self) -> &str {
                        &self.unique_id
                    }
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

        impl $crate::client::Sender {
            pub(crate) fn add_to(self, storage: &mut Devices) -> anyhow::Result<()> {
                $(
                    rpc!(@if_specific $trait_name {
                        if self.device_type == stringify!($trait_name) {
                            return Ok(<Self as $trait_name>::add_to(self, storage));
                        }
                    });
                )*
                anyhow::bail!("Unknown device type {}", self.device_type)
            }
        }

        impl Devices {
            pub(crate) fn stream_configured(&self) -> impl '_ + futures::Stream<Item = ConfiguredDevice> {
                async_stream::stream! {
                    $(
                        rpc!(@if_specific $trait_name {
                            for (device_number, device) in self.$trait_name.iter().enumerate() {
                                let device = device.lock().await;
                                let device = ConfiguredDevice {
                                    device_name: device.name().await.unwrap_or_default(),
                                    device_type: stringify!($trait_name).to_owned(),
                                    device_number,
                                    unique_id: device.unique_id().to_owned(),
                                };
                                tracing::debug!(?device, "Reporting device");
                                yield device;
                            }
                        });
                    )*
                }
            }

            pub(crate) async fn handle_action(&self, device_type: &str, device_number: usize, is_mut: bool, action: &str, params: $crate::params::OpaqueParams) -> axum::response::Result<$crate::ASCOMResult<$crate::response::OpaqueResponse>> {
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
