macro_rules! auto_increment {
    () => {{
        use std::sync::atomic::{AtomicU32, Ordering};

        static COUNTER: AtomicU32 = AtomicU32::new(1);
        std::num::NonZeroU32::new(COUNTER.fetch_add(1, Ordering::Relaxed)).unwrap()
    }};
}

pub(crate) use auto_increment;

#[cfg_attr(
    not(all(feature = "client", feature = "server", feature = "camera")),
    allow(unused_macro_rules)
)]
macro_rules! rpc_trait {
    (@extras Device trait) => {
        /// Static device name for the configured list.
        fn static_name(&self) -> &str;

        /// Unique ID of this device.
        fn unique_id(&self) -> &str;

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
        fn setup(&self) -> futures::future::BoxFuture<'_, eyre::Result<String>> {
            Box::pin(futures::future::ok(include_str!("../server/device_setup_template.html").to_owned()))
        }
    };

    (@extras Device client) => {
        fn static_name(&self) -> &str {
            &self.name
        }

        fn unique_id(&self) -> &str {
            &self.unique_id
        }

        fn setup(&self) -> futures::future::BoxFuture<'_, eyre::Result<String>> {
            Box::pin(async move {
                Ok(
                    $crate::client::REQWEST
                        .get(self.inner.base_url.join("setup")?)
                        .send()
                        .await?
                        .text()
                        .await?
                )
            })
        }
    };

    (@extras Device mod) => {};
    (@extras $trait_name:ident mod) => {
        impl $crate::api::devices_impl::RetrieavableDevice for dyn $trait_name {
            const TYPE: DeviceType = DeviceType::$trait_name;

            fn get_storage(storage: &Devices) -> &[std::sync::Arc<Self>] {
                &storage.$trait_name
            }
        }

        impl $crate::api::devices_impl::RegistrableDevice<dyn $trait_name> for std::sync::Arc<dyn $trait_name> {
            fn add_to(self, storage: &mut Devices) {
                storage.$trait_name.push(self);
            }
        }

        impl<T: 'static + $trait_name> $crate::api::devices_impl::RegistrableDevice<dyn $trait_name> for T {
            fn add_to(self, storage: &mut Devices) {
                storage.$trait_name.push(std::sync::Arc::new(self));
            }
        }

        #[cfg(test)]
        #[tokio::test]
        async fn run_proxy_tests() -> eyre::Result<()> {
            $crate::test::run_proxy_tests::<dyn $trait_name>().await
        }
    };

    // Don't add any extra code for other traits in other locations.
    (@extras $trait_name:ident $loc:ident) => {};

    (
        $(# $attr:tt)*
        $pub:vis trait $trait_name:ident: $trait_parents:ty {
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
        use serde::Serialize;

        #[cfg_attr(feature = "client", derive(Serialize), serde(untagged))]
        #[expect(non_camel_case_types)]
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

        #[cfg_attr(feature = "server", derive(Serialize), serde(untagged))]
        #[expect(non_camel_case_types)]
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
                            #[expect(unused)]
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

        $(# $attr)*
        #[async_trait::async_trait]
        #[allow(unused_variables)]
        $pub trait $trait_name: $trait_parents {
            $(
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
                ) -> $return_type $default_body
            )*

            rpc_trait!(@extras $trait_name trait);
        }

        #[cfg(feature = "client")]
        #[async_trait::async_trait]
        impl $trait_name for $crate::client::RawDeviceClient {
            $(
                $(# $method_attr)*
                async fn $method_name(
                    & $self $(, $param: $param_ty)*
                ) -> $return_type {
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

            rpc_trait!(@extras $trait_name client);
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
        impl Action {
            pub(super) async fn handle(self, device: &dyn $trait_name) -> ASCOMResult<Response> {
                match self {
                    $(
                        Self::$method_name { $($param),* } => {
                            device.$method_name($($param),*).await.map(Response::$method_name)
                        }
                    )*
                }
            }
        }

        rpc_trait!(@extras $trait_name mod);
    }

    pub use [<$trait_name:snake>]::$trait_name;
    });
}

pub(crate) use rpc_trait;

macro_rules! rpc_mod {
    ($($trait_name:ident = $path:literal,)*) => (paste::paste! {
        #[derive(PartialOrd, Ord, PartialEq, Eq, Clone, Copy, Debug, derive_more::Display, serde::Serialize, serde::Deserialize)]
        pub(crate) enum DeviceType {
            $(
                #[cfg(feature = $path)]
                #[display($path)]
                $trait_name,
            )*
        }

        /// A tagged enum wrapper for a type-erased instance of a device.
        #[derive(Clone, Debug)]
        #[expect(missing_docs)]
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

        #[cfg(feature = "client")]
        #[derive(Deserialize)]
        #[serde(untagged)]
        pub(crate) enum FallibleDeviceType {
            Known(DeviceType),
            Unknown(String),
        }

        #[cfg(feature = "server")]
        #[derive(Deserialize)]
        #[serde(remote = "DeviceType")]
        pub(crate) enum DevicePath {
            $(
                #[cfg(feature = $path)]
                #[serde(rename = $path)]
                $trait_name,
            )*
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
        #[expect(non_snake_case)]
        #[derive(Clone)]
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
                        _ = f.field(stringify!($trait_name), &self.$trait_name);
                    }
                )*
                f.finish()
            }
        }

        impl Devices {
            /// Create an empty collection of devices.
            ///
            /// Same as [`Default::default`] but works in const contexts.
            pub const fn default() -> Self {
                Self {
                    $(
                        #[cfg(feature = $path)]
                        $trait_name: Vec::new(),
                    )*
                }
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

        #[cfg(feature = "server")]
        impl Devices {
            pub(crate) fn get_device_for_server(
                &self,
                device_type: DeviceType,
                device_number: usize,
            ) -> $crate::server::Result<&dyn Device> {
                // With trait upcasting, we can get any device as dyn Device directly
                Ok(match device_type {
                    $(
                        #[cfg(feature = $path)]
                        DeviceType::$trait_name => {
                            self.get_for_server::<dyn $trait_name>(device_number)?
                        }
                    )*
                })
            }

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
                    TypedDeviceAction::Device(action) => {
                        let device = self.get_device_for_server(device_type, device_number)?;
                        TypedResponse::Device(action.handle(device).await?)
                    }
                })
            }
        }
    });
}

pub(crate) use rpc_mod;
