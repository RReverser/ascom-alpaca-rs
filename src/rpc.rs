macro_rules! rpc {
    (@if_specific Device $then:tt $({ $($else:tt)* })?) => {
        $($($else)*)?
    };

    (@if_specific $trait_name:ident { $($then:tt)* } $($else:tt)?) => {
        $($then)*
    };

    (@is_mut mut $self:ident) => (true);

    (@is_mut $self:ident) => (false);

    (@params_pat_impl $variant:ident, $($mut_device:ident)+, $params:ident) => ($crate::params::DeviceActionParams::$variant {
        $($mut_device)+,
        mut $params
    });

    (@params_pat mut $self:ident, $device:ident, $params:ident) => (rpc!(@params_pat_impl Put, mut $device, $params));

    (@params_pat $self:ident, $device:ident, $params:ident) => (rpc!(@params_pat_impl Get, $device, $params));

    (@get_self mut $self:ident) => ($self);

    (@get_self $self:ident) => ($self);

    (@storage $($trait_name:ident = $path:literal,)*) => {
        #[derive(Deserialize, PartialEq, Eq, Clone, Copy)]
        pub enum DeviceType {
            $(
                #[cfg(feature = $path)]
                $trait_name,
            )*
        }

        impl DeviceType {
            const fn as_str(self) -> &'static str {
                match self {
                    $(
                        #[cfg(feature = $path)]
                        DeviceType::$trait_name => stringify!($trait_name),
                    )*
                }
            }
        }

        impl std::fmt::Display for DeviceType {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl std::fmt::Debug for DeviceType {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                std::fmt::Display::fmt(self, f)
            }
        }

        impl Serialize for DeviceType {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                self.as_str().serialize(serializer)
            }
        }

        #[derive(PartialEq, Eq, Clone, Copy)]
        pub(crate) struct DevicePath(pub(crate) DeviceType);

        impl DevicePath {
            const fn as_str(self) -> &'static str {
                match self.0 {
                    $(
                        #[cfg(feature = $path)]
                        DeviceType::$trait_name => $path,
                    )*
                }
            }
        }

        impl std::fmt::Display for DevicePath {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl std::fmt::Debug for DevicePath {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                std::fmt::Display::fmt(self, f)
            }
        }

        impl<'de> Deserialize<'de> for DevicePath {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                Ok(DevicePath(match String::deserialize(deserializer)?.as_str() {
                    $(
                        #[cfg(feature = $path)]
                        $path => DeviceType::$trait_name,
                    )*
                    other => return Err(serde::de::Error::unknown_variant(other, &[ $(
                        #[cfg(feature = $path)]
                        $path
                    ),* ])),
                }))
            }
        }

        #[allow(non_snake_case)]
        #[derive(Default)]
        pub struct Devices {
            $(
                #[cfg(feature = $path)]
                $trait_name: Vec<std::sync::Arc<tokio::sync::RwLock<dyn $trait_name>>>,
            )*
        }

        impl std::fmt::Debug for Devices {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let mut f = f.debug_struct("Devices");
                $(
                    #[cfg(feature = $path)]
                    if !self.$trait_name.is_empty() {
                        let _ = f.field(stringify!($trait_name), &self.$trait_name);
                    }
                )*
                f.finish()
            }
        }

        impl Devices {
            pub(crate) async fn handle_action(&self, device_type: DeviceType, device_number: usize, action: &str, params: $crate::params::RawActionParams) -> axum::response::Result<$crate::ASCOMResult<$crate::response::OpaqueResponse>> {
                match device_type {
                    $(
                        #[cfg(feature = $path)]
                        DeviceType::$trait_name => {
                            let device = <dyn $trait_name>::get_in(self, device_number)?;
                            let params = $crate::params::DeviceActionParams::new(device, params).await;
                            <dyn $trait_name>::handle_action(action, params).await
                        }
                    )*
                }
            }
        }

        impl $crate::client::Sender {
            pub(crate) fn add_to(self, storage: &mut Devices) {
                match self.device_type {
                    $(
                        #[cfg(feature = $path)]
                        DeviceType::$trait_name => <Self as $trait_name>::add_to(self, storage),
                    )*
                }
            }
        }

        $(
            #[cfg(feature = $path)]
            impl dyn $trait_name {
                pub(crate) fn get_in(storage: &Devices, device_number: usize) -> axum::response::Result<&tokio::sync::RwLock<dyn $trait_name>> {
                    match storage.$trait_name.get(device_number) {
                        Some(device) => Ok(device),
                        None => Err((axum::http::StatusCode::NOT_FOUND, concat!(stringify!($trait_name), " not found")).into()),
                    }
                }
            }
        )*

        #[derive(Debug, Serialize, Deserialize)]
        #[serde(rename_all = "PascalCase")]
        pub struct ConfiguredDevice {
            pub device_name: String,
            pub device_type: DeviceType,
            pub device_number: usize,
            #[serde(rename = "UniqueID")]
            pub unique_id: String,
        }

        #[derive(Debug, Serialize, Deserialize)]
        #[serde(rename_all = "PascalCase")]
        pub struct ServerInfo {
            pub server_name: String,
            pub manufacturer: String,
            pub manufacturer_version: String,
            pub location: String,
        }

        #[macro_export]
        macro_rules! CargoServerInfo {
            () => ($crate::api::ServerInfo {
                server_name: env!("CARGO_PKG_NAME").to_owned(),
                manufacturer: env!("CARGO_PKG_AUTHORS").to_owned(),
                manufacturer_version: env!("CARGO_PKG_VERSION").to_owned(),
                location: env!("CARGO_PKG_HOMEPAGE").to_owned(),
            });
        }

        impl Devices {
            pub(crate) fn stream_configured(&self) -> impl '_ + futures::Stream<Item = ConfiguredDevice> {
                async_stream::stream! {
                    $(
                        #[cfg(feature = $path)]
                        for (device_number, device) in self.$trait_name.iter().enumerate() {
                            let device = device.read().await;
                            let device = ConfiguredDevice {
                                device_name: device.name().await.unwrap_or_default(),
                                device_type: DeviceType::$trait_name,
                                device_number,
                                unique_id: device.unique_id().to_owned(),
                            };
                            tracing::debug!(?device, "Reporting device");
                            yield device;
                        }
                    )*
                }
            }
        }
    };

    (@trait $(#[doc = $doc:literal])* $(#[http($path:literal)])? $trait_name:ident: $($parent:path),* {
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
            async fn handle_action(action: &str, params: $crate::params::DeviceActionParams<'_, impl ?Sized + $trait_name>) -> axum::response::Result<$crate::ASCOMResult<$crate::response::OpaqueResponse>> {
                #[allow(unused)]
                match (action, params) {
                    $(
                        ($method_path, rpc!(@params_pat $($mut_self)*, device, params)) => {
                            $(
                                let $param = params.extract($param_query).map_err(|err| (axum::http::StatusCode::BAD_REQUEST, format!("{err:#}")))?;
                            )*
                            Ok(device.$method_name($($param),*).await.map($crate::response::OpaqueResponse::new))
                        },
                    )*
                    (action, params) => rpc!(@if_specific $trait_name {
                        <dyn Device>::handle_action(action, params).await
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
        $(#[http($path:literal)])?
        pub trait $trait_name:ident $trait_body:tt
    )*) => {
        rpc!(@storage $($($trait_name = $path,)?)*);

        $(
            $(#[cfg(feature = $path)])?
            rpc!(@if_specific $trait_name {
                rpc!(@trait $(#[doc = $doc])* $(#[http($path)])? $trait_name: Device, Send, Sync $trait_body {
                    /// Register this device in the storage.
                    /// This method should not be overridden by implementors.
                    fn add_to(self, storage: &mut Devices) where Self: Sized + 'static {
                        storage.$trait_name.push(std::sync::Arc::new(tokio::sync::RwLock::new(self)));
                    }
                } {});
            } {
                rpc!(@trait $(#[doc = $doc])* $(#[http($path)])? $trait_name: std::fmt::Debug, Send, Sync $trait_body {
                    /// Unique ID of this device, ideally UUID.
                    fn unique_id(&self) -> &str;
                } {
                    fn unique_id(&self) -> &str {
                        &self.unique_id
                    }
                });
            });
        )*
    };
}

pub(crate) use rpc;
