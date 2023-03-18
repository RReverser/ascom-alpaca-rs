macro_rules! rpc {
    (@if_specific Device $then:tt $({ $($else:tt)* })?) => {
        $($($else)*)?
    };

    (@if_specific $trait_name:ident { $($then:tt)* } $($else:tt)?) => {
        $($then)*
    };

    (@is_mut mut $self:ident) => (true);

    (@is_mut $self:ident) => (false);

    (@params_pat_impl $variant:ident, $params:ident) => ($crate::params::ActionParams::$variant(mut $params));

    (@params_pat mut $self:ident, $params:ident) => (rpc!(@params_pat_impl Put, $params));

    (@params_pat $self:ident, $params:ident) => (rpc!(@params_pat_impl Get, $params));

    (@device_lock mut $self:ident) => (tokio::sync::RwLock::write);

    (@device_lock $self:ident) => (tokio::sync::RwLock::read);

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
                struct DebugRes<T, E>(Result<T, E>);

                impl<T: std::fmt::Debug, E: std::fmt::Debug> std::fmt::Debug for DebugRes<T, E> {
                    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        match &self.0 {
                            Ok(t) => t.fmt(f),
                            err => err.fmt(f),
                        }
                    }
                }

                struct DebugList<'list, T: ?Sized> {
                    list: &'list [std::sync::Arc<tokio::sync::RwLock<T>>]
                }

                impl<T: ?Sized + std::fmt::Debug> std::fmt::Debug for DebugList<'_, T> {
                    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        f.debug_list().entries(self.list.iter().map(|device| DebugRes(device.try_read()))).finish()
                    }
                }

                let mut f = f.debug_struct("Devices");
                $(
                    #[cfg(feature = $path)]
                    if !self.$trait_name.is_empty() {
                        let _ = f.field(stringify!($trait_name), &DebugList { list: &self.$trait_name });
                    }
                )*
                f.finish()
            }
        }

        pub trait RegistrableDevice<DynTrait: ?Sized + Device>: Device /* where Self: Unsize<DynTrait> */ {
            fn add_to(self, storage: &mut Devices);
        }

        impl RegistrableDevice<dyn Device> for $crate::client::Sender {
            fn add_to(self, storage: &mut Devices) {
                match self.device_type {
                    $(
                        #[cfg(feature = $path)]
                        DeviceType::$trait_name => storage.register::<dyn $trait_name>(self),
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

            #[cfg(feature = $path)]
            impl<T: 'static + $trait_name> RegistrableDevice<dyn $trait_name> for T {
                fn add_to(self, storage: &mut Devices) {
                    storage.$trait_name.push(std::sync::Arc::new(tokio::sync::RwLock::new(self)));
                }
            }
        )*

        impl Devices {
            pub fn register<DynTrait: ?Sized + Device>(&mut self, device: impl RegistrableDevice<DynTrait>) {
                device.add_to(self);
            }

            pub(crate) async fn handle_action(&self, device_type: DeviceType, device_number: usize, action: &str, params: $crate::params::ActionParams) -> axum::response::Result<$crate::ASCOMResult<$crate::response::OpaqueResponse>> {
                match device_type {
                    $(
                        #[cfg(feature = $path)]
                        DeviceType::$trait_name => {
                            let device = <dyn $trait_name>::get_in(self, device_number)?;
                            <dyn $trait_name>::handle_action(device, action, params).await
                        }
                    )*
                }
            }

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

    (@decode_response $resp:expr => ImageArrayResponse) => {
        Ok(std::convert::identity::<ImageArrayResponse>($resp))
    };

    (@decode_response $resp:expr => $return_type:ty) => {
        std::convert::identity::<$crate::response::OpaqueResponse>($resp)
        .try_as::<$return_type>()
        .map_err(|err| $crate::ASCOMError::new($crate::ASCOMErrorCode::UNSPECIFIED, format!("{err:#}")))
    };

    (@decode_response $resp:expr) => {{
        let _: $crate::response::OpaqueResponse = $resp;
        Ok(())
    }};

    (@trait $(#[doc = $doc:literal])* $(#[http($path:literal)])? $trait_name:ident: $($first_parent:ident)::+ $(+ $($other_parents:ident)::+)* {
        $(#[doc = $docs_before_methods:literal])*

        $(
            #[extra_method(client_impl = $client_impl:expr)]
            fn $extra_method_name:ident (& $($extra_mut_self:ident)+ $(, $extra_param:ident: $extra_param_ty:ty)* $(,)?) $(-> $extra_method_return:ty)?;

            $(#[doc = $docs_after_extra_method:literal])*
        )*

        $(
            #[http($method_path:literal)]
            fn $method_name:ident(& $($mut_self:ident)* $(, #[http($param_query:literal)] $param:ident: $param_ty:ty)* $(,)?) $(-> $return_type:ty)?;

            $(#[doc = $docs_after_method:literal])*
        )*
    }) => {
        $(#[cfg(feature = $path)])?
        #[allow(unused_variables)]
        $(#[doc = $doc])*
        #[cfg_attr(not(all(doc, feature = "nightly")), async_trait::async_trait)]
        pub trait $trait_name: $($first_parent)::+ $(+ $($other_parents)::+)* {
            $(#[doc = $docs_before_methods])*

            $(
                fn $extra_method_name (& $($extra_mut_self)+ $(, $extra_param: $extra_param_ty)*) $(-> $extra_method_return)?;

                $(#[doc = $docs_after_extra_method])*
            )*

            $(
                async fn $method_name(& $($mut_self)* $(, $param: $param_ty)*) -> $crate::ASCOMResult$(<$return_type>)? {
                    Err($crate::ASCOMError::NOT_IMPLEMENTED)
                }

                $(#[doc = $docs_after_method])*
            )*
        }

        $(#[cfg(feature = $path)])?
        impl dyn $trait_name {
            /// Private inherent method for handling actions.
            /// This method could live on the trait itself, but then it wouldn't be possible to make it private.
            async fn handle_action(device: &tokio::sync::RwLock<impl ?Sized + $trait_name>, action: &str, params: $crate::params::ActionParams) -> axum::response::Result<$crate::ASCOMResult<$crate::response::OpaqueResponse>> {
                #[allow(unused)]
                match (action, params) {
                    $(
                        ($method_path, rpc!(@params_pat $($mut_self)*, params)) => {
                            $(
                                let $param = params.extract($param_query).map_err(|err| (axum::http::StatusCode::BAD_REQUEST, format!("{err:#}")))?;
                            )*
                            Ok(rpc!(@device_lock $($mut_self)*)(device).await.$method_name($($param),*).await.map($crate::response::OpaqueResponse::new))
                        },
                    )*
                    (action, params) => rpc!(@if_specific $trait_name {
                        <dyn Device>::handle_action(device, action, params).await
                    } {
                        Err((axum::http::StatusCode::NOT_FOUND, "Unknown action").into())
                    })
                }
            }
        }

        $(#[cfg(feature = $path)])?
        #[cfg_attr(not(all(doc, feature = "nightly")), async_trait::async_trait)]
        impl $trait_name for $crate::client::Sender {
            $(
                fn $extra_method_name (& $($extra_mut_self)+ $(, $extra_param: $extra_param_ty)*) $(-> $extra_method_return)? {
                    $client_impl
                }
            )*

            $(
                async fn $method_name(& $($mut_self)* $(, $param: $param_ty)*) -> $crate::ASCOMResult$(<$return_type>)? {
                    #[allow(unused_mut)]
                    let mut opaque_params = $crate::params::OpaqueParams::default();
                    $(
                        opaque_params.insert($param_query, $param);
                    )*
                    rpc!(@decode_response
                        rpc!(@get_self $($mut_self)*).exec_action(rpc!(@is_mut $($mut_self)*), $method_path, opaque_params).await?
                        $(=> $return_type)?
                    )
                }
            )*
        }
    };

    ($(
        $(#[doc = $doc:literal])*
        $(#[http($path:literal)])?
        pub trait $trait_name:ident: $($first_parent:ident)::+ $(+ $($other_parents:ident)::+)* { $($trait_body:tt)* }
    )*) => {
        rpc!(@storage $($($trait_name = $path,)?)*);

        $(
            $(#[cfg(any(feature = $path, doc))])?
            rpc!(@trait $(#[doc = $doc])* $(#[http($path)])? $trait_name: $($first_parent)::+ $(+ $($other_parents)::+)* { $($trait_body)* });
        )*
    };
}

pub(crate) use rpc;
