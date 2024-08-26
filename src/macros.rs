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

    (
        $(# $attr:tt)*
        $pub:vis trait $trait_name:ident: $($first_parent:ident)::+ $(+ $($other_parents:ident)::+)* {
            $(
                const EXTRA_METHODS: () = {
                    $(
                        $(#[doc = $extra_method_doc:literal])*
                        $($extra_method_name:ident)+ ($($extra_method_params:tt)*) $(-> $extra_method_return:ty)? $extra_method_client_impl:block
                    )*
                };
            )?

            $(
                $(#[doc = $doc:literal])*
                #[http($method_path:literal, method = $http_method:ident $(, via = $via:ident)?)]
                async fn $method_name:ident(
                    & $self:ident $(, #[http($param_query:literal $(, via = $param_via:ident)?)] $param:ident: $param_ty:ty)* $(,)?
                ) -> $return_type:ty $default_body:block
            )*
        }
    ) => {
        $(# $attr)*
        #[async_trait::async_trait]
        #[allow(unused_variables)]
        $pub trait $trait_name: $($first_parent)::+ $(+ $($other_parents)::+)* {
            $(
                $(
                    $(#[doc = $extra_method_doc])*
                    $($extra_method_name)+ ($($extra_method_params)*) $(-> $extra_method_return)?;
                )*
            )?

            $(
                $(#[doc = $doc])*
                ///
                /// Definition before the `#[async_trait]` expansion:
                ///
                /// ```ignore
                #[doc = concat!("async fn ", stringify!($method_name), "(&self", $(", ", stringify!($param), ": ", stringify!($param_ty),)* ") -> ", stringify!($return_type))]
                /// # { unimplemented!() }
                /// ```
                async fn $method_name(
                    & $self $(, $param: $param_ty)*
                ) -> $return_type $default_body
            )*
        }

        impl PartialEq for dyn $trait_name {
            fn eq(&self, other: &Self) -> bool {
                self.unique_id() == other.unique_id()
            }
        }

        impl Eq for dyn $trait_name {}

        impl std::hash::Hash for dyn $trait_name {
            fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
                self.unique_id().hash(state);
            }
        }

        #[cfg(feature = "server")]
        impl dyn $trait_name {
            /// Private inherent method for handling actions.
            /// This method could live on the trait itself, but then it wouldn't be possible to make it private.
            #[allow(non_camel_case_types)]
            async fn handle_action(device: &(impl ?Sized + $trait_name), action: &str, params: $crate::server::ActionParams) -> $crate::server::Result<impl Serialize> {
                #[derive(Serialize)]
                #[serde(untagged)]
                #[allow(non_camel_case_types)]
                enum ResponseRepr<$($method_name),*> {
                    $($method_name($method_name),)*
                }

                let value = match (action, params) {
                    $(
                        ($method_path, $crate::server::ActionParams::$http_method(params)) => {
                            #[allow(unused_mut)]
                            let mut params = params;
                            $(
                                let $param =
                                    params.extract($param_query)
                                    $(.map($param_via::into))?
                                    ?;
                            )*
                            params.finish_extraction();

                            let value =
                                device
                                .$method_name($($param),*)
                                .await
                                $(.map($via::from))?
                                ?;

                            ResponseRepr::$method_name(value)
                        }
                    )*
                    (action, params) => rpc_trait!(@if_specific $trait_name {
                        return match <dyn Device>::handle_action(device, action, params).await {
                            Ok(value) => Ok($crate::either::Either::Right(value)),
                            Err(mut err) => {
                                if let $crate::server::Error::UnknownAction { device_type, .. } = &mut err {
                                    // Update to a more specific device type.
                                    *device_type = stringify!($trait_name);
                                }
                                Err(err)
                            }
                        };
                    } {
                        let _ = params;
                        return Err($crate::server::Error::UnknownAction {
                            device_type: stringify!($trait_name),
                            action: action.to_owned(),
                        });
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
        #[async_trait::async_trait]
        impl $trait_name for $crate::client::RawDeviceClient {
            $(
                $(
                    $($extra_method_name)+ ($($extra_method_params)*) $(-> $extra_method_return)? $extra_method_client_impl
                )*
            )?

            $(
                #[allow(non_camel_case_types)]
                async fn $method_name(& $self $(, $param: $param_ty)*) -> $return_type {
                    #[derive(Debug, Serialize)]
                    struct OpaqueParams<$($param),*> {
                        $(
                            #[serde(rename = $param_query)]
                            $param: $param,
                        )*
                    }

                    $self
                    .exec_action($crate::client::ActionParams {
                        action: $method_path,
                        method: $crate::client::Method::$http_method,
                        params: OpaqueParams {
                            $($param $(: $param_via::from($param))?),*
                        }
                    })
                    .await
                    $(.map($via::into))?
                }
            )*
        }
    };
}

pub(crate) use rpc_trait;

macro_rules! rpc_mod {
    ($($trait_name:ident = $path:literal,)*) => {
        pub(crate) mod internal {
            // Not really public, needed for a Voldemort trait RetrieavableDevice.
            #[derive(PartialOrd, Ord, PartialEq, Eq, Clone, Copy)]
            pub enum DeviceType {
                $(
                    #[cfg(feature = $path)]
                    $trait_name,
                )*
            }
        }
        pub(crate) use internal::DeviceType;

        #[allow(missing_docs)]
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
                match *self {
                    $(
                        #[cfg(feature = $path)]
                        Self::$trait_name(ref device) => $crate::api::ConfiguredDevice {
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

        /// Devices collection.
        ///
        /// This data structure holds devices of arbitrary categories (cameras, telescopes, etc.)
        /// and allows to register and access them by their kind and index.
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
            /// Iterate over all devices of a given type.
            pub fn iter<DynTrait: ?Sized + $crate::api::devices_impl::RetrieavableDevice>(&self) -> impl '_ + Iterator<Item = std::sync::Arc<DynTrait>> {
                DynTrait::get_storage(self).iter().map(std::sync::Arc::clone)
            }

            /// Iterate over all registered devices.
            ///
            /// The second element of the tuple is the index of the device within its category
            /// (not the whole collection).
            //
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
            pub(crate) async fn handle_action<'this>(&'this self, device_type: DeviceType, device_number: usize, action: &'this str, params: $crate::server::ActionParams) -> $crate::server::Result<impl 'this + Serialize> {
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
