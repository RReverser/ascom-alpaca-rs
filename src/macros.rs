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
    (@finish Device $($rest:tt)*) => {
        rpc_trait!(
            @finish

            _Device

            $($rest)*

            {
                /// Static device name for the configured list.
                fn static_name(&self) -> &str
            }
            ;
            {
                &self.name
            }

            {
                /// Unique ID of this device.
                fn unique_id(&self) -> &str
            }
            ;
            {
                &self.unique_id
            }

            {
                /// Web page user interface that enables device specific configuration to be set for each available device.
                ///
                /// The server should implement this to return HTML string. You can use [`Self::action`] to store the configuration.
                ///
                /// Note: on the client side you almost never want to just retrieve HTML to show it in the browser, as that breaks relative URLs.
                /// Use the `/{device_type}/{device_number}/setup` URL instead.
                ///
                /// Definition before the `#[async_trait]` expansion:
                /// ```ignore
                /// async fn setup(&self) -> eyre::Result<String>
                /// # { unimplemented!() }
                /// ```
                async fn setup(&self) -> eyre::Result<String>
            }
            {
                Ok(include_str!("../server/device_setup_template.html").to_owned())
            }
            {
                Ok(
                    $crate::client::REQWEST
                        .get(self.inner.base_url.join("setup")?)
                        .send()
                        .await?
                        .text()
                        .await?
                )
            }
        );
    };
    (@finish $trait_name:ident { $($trait_header:tt)* } { $($client_header:tt)* } $({ $($method_header:tt)* } $trait_body:tt $client_body:tt)*) => {
        $($trait_header)* {
            $(
                $($method_header)* $trait_body
            )*
        }

        $($client_header)* {
            $(
                $($method_header)* $client_body
            )*
        }
    };

    (
        $(# $attr:tt)*
        $pub:vis trait $trait_name:ident: $($first_parent:ident)::+ $(+ $($other_parents:ident)::+)* {
            $(
                $(#[doc = $doc:literal])*
                #[http($method_path:literal, method = $http_method:ident $(, via = $via:path)?)]
                $(# $method_attr:tt)*
                async fn $method_name:ident(
                    & $self:ident $(, #[http($param_query:literal $(, via = $param_via:path)?)] $param:ident: $param_ty:ty)* $(,)?
                ) -> $return_type:ty $default_body:block
            )*
        }
    ) => (paste::paste! {
    $(# $attr)*
    pub(crate) mod [<$trait_name:snake>] {
        use super::*;

        #[cfg_attr(feature = "client", derive(serde::Serialize), serde(untagged))]
        #[allow(non_camel_case_types)]
        pub(super) enum Action {
            $(
                $method_name {
                    $(
                        #[cfg_attr(feature = "client", serde(rename = $param_query))]
                        $param: $param_ty,
                    )*
                },
            )*
        }

        #[cfg_attr(feature = "server", derive(serde::Serialize), serde(untagged))]
        #[allow(non_camel_case_types, unused_parens)]
        pub(super) enum Response {
            $(
                $method_name(<$return_type as $crate::errors::ASCOMResultOk>::Ok),
            )*
        }

        impl $crate::params::Action for Action {
            #[cfg(feature = "server")]
            fn from_parts(action: &str, params: &mut $crate::server::ActionParams) -> $crate::server::Result<Option<Self>> {
                Ok(Some(match (action, params) {
                    $(
                        ($method_path, $crate::server::ActionParams::$http_method(params)) => {
                            #[allow(unused)]
                            let mut params = params;
                            $(
                                let $param =
                                    params.extract($param_query)
                                    $(.map(<$param_via>::into))?
                                    ?;
                            )*

                            Self::$method_name { $($param),* }
                        }
                    )*
                    _ => return Ok(None),
                }))
            }

            #[cfg(feature = "client")]
            fn into_parts(self) -> $crate::params::ActionParams<impl serde::Serialize> {
                let (method, action) = match self {
                    $(Self::$method_name { .. } => ($crate::params::Method::$http_method, $method_path),)*
                };

                $crate::params::ActionParams {
                    action,
                    method,
                    params: self,
                }
            }
        }

        rpc_trait!(
            @finish
            $trait_name
            {
                $(# $attr)*
                #[async_trait::async_trait]
                #[allow(unused_variables)]
                $pub trait $trait_name: $($first_parent)::+ $(+ $($other_parents)::+)*
            }
            {
                #[cfg(feature = "client")]
                #[async_trait::async_trait]
                impl $trait_name for $crate::client::RawDeviceClient
            }

            $(
                {
                    $(#[doc = $doc])*
                    ///
                    /// Definition before the `#[async_trait]` expansion:
                    ///
                    /// ```ignore
                    #[doc = concat!("async fn ", stringify!($method_name), "(&self", $(", ", stringify!($param), ": ", stringify!($param_ty),)* ") -> ", stringify!($return_type))]
                    /// # { unimplemented!() }
                    /// ```
                    $(# $method_attr)*
                    async fn $method_name(
                        & $self $(, $param: $param_ty)*
                    ) -> $return_type
                }
                $default_body
                {
                    $self
                    .exec_action(Action::$method_name {
                        $(
                            $param,
                        )*
                    })
                    .await
                    $(.map(<$via>::into))?
                }
            )*
        );

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
        impl Action {
            pub(super) async fn handle(self, device: &(impl ?Sized + $trait_name)) -> ASCOMResult<Response> {
                match self {
                    $(
                        Self::$method_name { $($param),* } => {
                            #[allow(deprecated)]
                            device.$method_name($($param),*).await.map(Response::$method_name)
                        }
                    )*
                }
            }
        }
    }

    pub use [<$trait_name:snake>]::$trait_name;
    });
}

pub(crate) use rpc_trait;

macro_rules! rpc_mod {
    ($($trait_name:ident = $path:literal,)*) => (paste::paste! {
        #[derive(PartialOrd, Ord, PartialEq, Eq, Clone, Copy)]
        pub(crate) enum DeviceType {
            $(
                #[cfg(feature = $path)]
                $trait_name,
            )*
        }

        /// A tagged enum wrapper for a type-erased instance of a device.
        #[derive(Clone, Debug)]
        #[allow(missing_docs)]
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
                        Self::$trait_name(ref device) => device.to_configured_device(as_number),
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

        impl<'de> Deserialize<'de> for $crate::api::devices_impl::FallibleDeviceType {
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

                Ok($crate::api::devices_impl::FallibleDeviceType(match MaybeDeviceType::deserialize(deserializer)? {
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

        impl $crate::api::devices_impl::DevicePath {
            const fn as_str(self) -> &'static str {
                match self.0 {
                    $(
                        #[cfg(feature = $path)]
                        DeviceType::$trait_name => $path,
                    )*
                }
            }
        }

        impl<'de> Deserialize<'de> for $crate::api::devices_impl::DevicePath {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                #[derive(Deserialize)]
                #[serde(remote = "DeviceType")]
                enum DevicePathRepr {
                    $(
                        #[cfg(feature = $path)]
                        #[serde(rename = $path)]
                        $trait_name,
                    )*
                }

                DevicePathRepr::deserialize(deserializer).map(Self)
            }
        }

        enum TypedDeviceAction {
            Device(device::Action),
            $(
                #[cfg(feature = $path)]
                $trait_name([<$trait_name:snake>]::Action),
            )*
        }

        #[cfg_attr(feature = "server", derive(serde::Serialize), serde(untagged))]
        enum TypedResponse {
            Device(device::Response),
            $(
                #[cfg(feature = $path)]
                $trait_name([<$trait_name:snake>]::Response),
            )*
        }

        impl TypedDeviceAction {
            #[cfg(feature = "server")]
            fn from_parts(device_type: DeviceType, action: &str, mut params: crate::server::ActionParams) -> crate::server::Result<Self> {
                let result = match device_type {
                    $(
                        #[cfg(feature = $path)]
                        DeviceType::$trait_name =>
                            $crate::params::Action::from_parts(action, &mut params)?
                            .map(Self::$trait_name),
                    )*
                };

                let result = match result {
                    Some(result) => result,
                    // Fallback to generic device actions.
                    None => {
                        $crate::params::Action::from_parts(action, &mut params)?
                        .map(Self::Device)
                        .ok_or_else(|| crate::server::Error::UnknownAction {
                            device_type,
                            action: action.to_owned(),
                        })?
                    }
                };

                params.finish_extraction();

                Ok(result)
            }
        }

        /// Devices collection.
        ///
        /// This data structure holds devices of arbitrary categories (cameras, telescopes, etc.)
        /// and allows to register and access them by their kind and index.
        #[allow(non_snake_case)]
        #[derive(Clone, Default)]
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

        impl Devices {
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

        #[cfg(feature = "server")]
        impl Devices {
            pub(crate) async fn handle_action<'this>(&'this self, device_type: DeviceType, device_number: usize, action: &'this str, params: $crate::server::ActionParams) -> $crate::server::Result<impl Serialize> {
                let action = TypedDeviceAction::from_parts(device_type, action, params)?;

                Ok(match action {
                    $(
                        #[cfg(feature = $path)]
                        TypedDeviceAction::$trait_name(action) => {
                            let device = self.get_for_server::<dyn $trait_name>(device_number)?;
                            TypedResponse::$trait_name(action.handle(device).await?)
                        }
                    )*
                    TypedDeviceAction::Device(action) => TypedResponse::Device(match device_type {
                        $(
                            #[cfg(feature = $path)]
                            DeviceType::$trait_name => action.handle(self.get_for_server::<dyn $trait_name>(device_number)?).await,
                        )*
                    }?)
                })
            }

            pub(crate) async fn get_setup_html(&self, device_type: DeviceType, device_number: usize) -> eyre::Result<String> {
                match device_type {
                    $(
                        #[cfg(feature = $path)]
                        DeviceType::$trait_name => self.get_for_server::<dyn $trait_name>(device_number)?.setup().await,
                    )*
                }
            }
        }

        #[cfg(test)]
        mod conformu {
            use super::DeviceType;
            use $crate::test_utils::ConformU;

            $(
                #[cfg(feature = $path)]
                #[serial_test::serial($trait_name)]
                #[allow(non_snake_case)]
                mod $trait_name {
                    use super::*;

                    #[tokio::test]
                    async fn alpaca() -> eyre::Result<()> {
                        ConformU::AlpacaProtocol.run_proxy_test(DeviceType::$trait_name).await
                    }

                    #[tokio::test]
                    async fn conformance() -> eyre::Result<()> {
                        ConformU::Conformance.run_proxy_test(DeviceType::$trait_name).await
                    }
                }
            )*
        }
    });
}

pub(crate) use rpc_mod;
