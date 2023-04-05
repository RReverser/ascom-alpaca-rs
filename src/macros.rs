macro_rules! auto_increment {
    () => {{
        static COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(1);
        std::num::NonZeroU32::new(COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
            .unwrap()
    }};
}

pub(crate) use auto_increment;

#[cfg_attr(
    not(all(feature = "client", feature = "server", feature = "camera")),
    allow(unused_macro_rules)
)]
macro_rules! rpc_trait {
    (@if_specific Device $then:block $else:block) => ($else);

    (@if_specific $trait_name:ident $then:block $else:block) => ($then);

    (@params_pat $scope:ident, mut $self:ident $inner:tt) => ($crate::$scope::ActionParams::Put $inner);

    (@params_pat $scope:ident, $self:ident $inner:tt) => ($crate::$scope::ActionParams::Get $inner);

    (
        $(# $attr:tt)*
        $pub:vis trait $trait_name:ident: $($first_parent:ident)::+ $(+ $($other_parents:ident)::+)* {
            $(#[doc = $docs_before_methods:literal])*

            $(
                #[extra_method(client_impl = $client_impl:expr)]
                fn $extra_method_name:ident (& $($extra_mut_self:ident)+ $(, $extra_param:ident: $extra_param_ty:ty)* $(,)?) $(-> $extra_method_return:ty)?;

                $(#[doc = $docs_after_extra_method:literal])*
            )*

            $(
                #[http($method_path:literal $(, via = $via:ident)?)]
                fn $method_name:ident(& $($mut_self:ident)* $(, #[http($param_query:ident)] $param:ident: $param_ty:ty)* $(,)?) $(-> $return_type:ty)?;

                $(#[doc = $docs_after_method:literal])*
            )*
        }
    ) => {
        $(# $attr)*
        #[cfg_attr(not(all(doc, feature = "nightly")), async_trait::async_trait)]
        #[allow(unused_variables)]
        $pub trait $trait_name: $($first_parent)::+ $(+ $($other_parents)::+)* {
            $(#[doc = $docs_before_methods])*

            $(
                fn $extra_method_name (& $($extra_mut_self)+ $(, $extra_param: $extra_param_ty)*) $(-> $extra_method_return)?;

                $(#[doc = $docs_after_extra_method])*
            )*

            $(
                async fn $method_name(&self $(, $param: $param_ty)*) -> $crate::ASCOMResult$(<$return_type>)? {
                    Err($crate::ASCOMError::NOT_IMPLEMENTED)
                }

                $(#[doc = $docs_after_method])*
            )*
        }

        #[cfg(feature = "server")]
        impl dyn $trait_name {
            /// Private inherent method for handling actions.
            /// This method could live on the trait itself, but then it wouldn't be possible to make it private.
            #[allow(non_camel_case_types)]
            async fn handle_action(device: &(impl ?Sized + $trait_name), action: &str, params: $crate::server::ActionParams) -> Result<impl Serialize, $crate::server::Error> {
                #[derive(Serialize)]
                #[serde(untagged)]
                enum ResponseRepr<$($method_name),*> {
                    $($method_name($method_name),)*
                }

                #[allow(unused)]
                let value = match (action, params) {
                    $(
                        ($method_path, rpc_trait!(@params_pat server, $($mut_self)* (mut params))) => {
                            $(
                                let $param = params.extract(stringify!($param_query)).map_err($crate::server::Error::BadRequest)?;
                            )*

                            let value =
                                device
                                .$method_name($($param),*)
                                .await?;

                            $(let value = $via::from(value);)?

                            ResponseRepr::$method_name(value)
                        }
                    )*
                    (action, params) => rpc_trait!(@if_specific $trait_name {
                        return <dyn Device>::handle_action(device, action, params).await.map($crate::either::Either::Right);
                    } {
                        return Err($crate::server::Error::NotFound(anyhow::anyhow!("Unknown action {}::{action}", stringify!($trait_name))));
                    })
                };

                Ok(rpc_trait!(@if_specific $trait_name {
                    $crate::either::Either::Left(value)
                } {
                    value
                }))
            }
        }

        #[cfg(feature = "client")]
        #[cfg_attr(not(all(doc, feature = "nightly")), async_trait::async_trait)]
        impl $trait_name for $crate::client::RawDeviceClient {
            $(
                fn $extra_method_name (& $($extra_mut_self)+ $(, $extra_param: $extra_param_ty)*) $(-> $extra_method_return)? {
                    $client_impl
                }
            )*

            $(
                async fn $method_name(&self $(, $param: $param_ty)*) -> $crate::ASCOMResult$(<$return_type>)? {
                    let opaque_params = $crate::client::opaque_params! {
                        $($param_query: $param,)*
                    };
                    self
                    .exec_action($method_path, rpc_trait!(@params_pat client, $($mut_self)* (opaque_params)))
                    .await
                    $(.map($via::into_inner))?
                }
            )*
        }
    };
}

pub(crate) use rpc_trait;

macro_rules! rpc_mod {
    ($($trait_name:ident = $path:literal,)*) => {
        #[cfg(not(any( $(feature = $path),* )))]
        compile_error!(concat!("At least one device type must be enabled via Cargo features:" $(, "\n - ", $path)*));

        pub(crate) mod internal {
            // Not really public, needed for a Voldemort trait RetrieavableDevice.
            #[derive(PartialEq, Eq, Clone, Copy)]
            pub enum DeviceType {
                $(
                    #[cfg(feature = $path)]
                    $trait_name,
                )*
            }
        }
        pub(crate) use internal::DeviceType;

        #[derive(Clone, Debug)]
        pub enum TypedDevice {
            $(
                #[cfg(feature = $path)]
                $trait_name(std::sync::Arc<dyn $trait_name>),
            )*
        }

        impl $crate::api::devices_impl::RegistrableDevice<dyn Device> for TypedDevice {
            fn add_to(self, storage: &mut Devices) {
                match self {
                    $(
                        #[cfg(feature = $path)]
                        Self::$trait_name(device) => storage.$trait_name.push(device),
                    )*
                }
            }
        }

        #[cfg(feature = "server")]
        impl TypedDevice {
            pub(crate) fn to_configured_device(&self, as_number: usize) -> $crate::api::ConfiguredDevice<DeviceType> {
                match self {
                    $(
                        #[cfg(feature = $path)]
                        Self::$trait_name(device) => $crate::api::ConfiguredDevice {
                            name: device.static_name().to_owned(),
                            ty: DeviceType::$trait_name,
                            number: as_number,
                            unique_id: device.unique_id().to_owned(),
                        },
                    )*
                }
            }
        }

        #[cfg(feature = "client")]
        impl $crate::client::RawDeviceClient {
            pub(crate) const fn into_typed_client(self: std::sync::Arc<Self>, device_type: DeviceType) -> TypedDevice {
                match device_type {
                    $(
                        #[cfg(feature = $path)]
                        DeviceType::$trait_name => TypedDevice::$trait_name(self),
                    )*
                }
            }
        }

        pub(crate) struct FallibleDeviceType(
            pub(crate) Result<DeviceType, String>,
        );

        impl std::fmt::Debug for FallibleDeviceType {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match &self.0 {
                    Ok(ty) => ty.fmt(f),
                    Err(ty) => write!(f, "Unsupported({})", ty),
                }
            }
        }

        impl<'de> Deserialize<'de> for FallibleDeviceType {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                #[derive(Deserialize)]
                #[serde(field_identifier)]
                enum MaybeDeviceType {
                    $(
                        #[cfg(feature = $path)]
                        $trait_name,
                    )*
                    Unknown(String),
                }

                Ok(FallibleDeviceType(match MaybeDeviceType::deserialize(deserializer)? {
                    $(
                        #[cfg(feature = $path)]
                        MaybeDeviceType::$trait_name => Ok(DeviceType::$trait_name),
                    )*
                    MaybeDeviceType::Unknown(s) => Err(s),
                }))
            }
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
                $trait_name: Vec<std::sync::Arc<dyn $trait_name>>,
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

        impl Extend<TypedDevice> for Devices {
            fn extend<T: IntoIterator<Item = TypedDevice>>(&mut self, iter: T) {
                for client in iter {
                    self.register(client);
                }
            }
        }

        impl FromIterator<TypedDevice> for Devices {
            fn from_iter<T: IntoIterator<Item = TypedDevice>>(iter: T) -> Self {
                let mut devices = Self::default();
                devices.extend(iter);
                devices
            }
        }

        impl Devices {
            pub fn iter<DynTrait: ?Sized + $crate::api::devices_impl::RetrieavableDevice>(&self) -> impl '_ + Iterator<Item = std::sync::Arc<DynTrait>> {
                DynTrait::get_storage(self).iter().map(std::sync::Arc::clone)
            }

            // TODO: make this IntoIterator (although the type is going to be ugly-looking).
            // The usize is returned as 2nd arg just to attract attention to it not being
            // a normal whole-iteration index.
            pub fn iter_all(&self) -> impl '_ + Iterator<Item = (TypedDevice, usize)> {
                let iter = std::iter::empty();

                $(
                    #[cfg(feature = $path)]
                    let iter = iter.chain(
                        self.iter::<dyn $trait_name>()
                        .map(TypedDevice::$trait_name)
                        .enumerate()
                        .map(|(typed_index, device)| (device, typed_index))
                    );
                )*

                iter
            }
        }

        $(
            #[cfg(feature = $path)]
            const _: () = {
                impl $crate::api::devices_impl::RetrieavableDevice for dyn $trait_name {
                    const TYPE: DeviceType = DeviceType::$trait_name;

                    fn get_storage(storage: &Devices) -> &[std::sync::Arc<Self>] {
                        &storage.$trait_name
                    }
                }

                impl<T: 'static + $trait_name> $crate::api::devices_impl::RegistrableDevice<dyn $trait_name> for T {
                    fn add_to(self, storage: &mut Devices) {
                        storage.$trait_name.push(std::sync::Arc::new(self));
                    }
                }
            };
        )*

        impl Devices {
            #[cfg(feature = "server")]
            pub(crate) async fn handle_action<'this>(&'this self, device_type: DeviceType, device_number: usize, action: &'this str, params: $crate::server::ActionParams) -> Result<impl 'this + Serialize, $crate::server::Error> {
                #[derive(Serialize)]
                #[serde(untagged)]
                enum ResponseRepr<$(
                    #[cfg(feature = $path)]
                    $trait_name,
                )*> {
                    $(
                        #[cfg(feature = $path)]
                        $trait_name($trait_name),
                    )*
                }

                Ok(match device_type {
                    $(
                        #[cfg(feature = $path)]
                        DeviceType::$trait_name => {
                            let device = self.get_for_server::<dyn $trait_name>(device_number)?;
                            let result = <dyn $trait_name>::handle_action(device, action, params).await?;
                            ResponseRepr::$trait_name(result)
                        }
                    )*
                })
            }
        }
    };
}

pub(crate) use rpc_mod;
