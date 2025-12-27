#[cfg(feature = "client")]
pub(crate) trait ConvertConvenienceProp {
    type Inner;
    type Arr;

    fn from_arr(arr: Self::Arr) -> Self;
    fn into_arr(self) -> Self::Arr;
}

#[cfg(feature = "client")]
const _: () = {
    impl<T> ConvertConvenienceProp for (T, T) {
        type Inner = T;
        type Arr = [T; 2];

        fn from_arr(arr: Self::Arr) -> Self {
            arr.into()
        }

        fn into_arr(self) -> Self::Arr {
            self.into()
        }
    }

    impl<T, const N: usize> ConvertConvenienceProp for [T; N] {
        type Inner = T;
        type Arr = [T; N];

        fn from_arr(arr: Self::Arr) -> Self {
            arr
        }

        fn into_arr(self) -> Self::Arr {
            self
        }
    }

    impl<T> ConvertConvenienceProp for std::ops::RangeInclusive<T> {
        type Inner = T;
        type Arr = [T; 2];

        fn from_arr([start, end]: Self::Arr) -> Self {
            start..=end
        }

        fn into_arr(self) -> Self::Arr {
            self.into_inner().into()
        }
    }
};

#[cfg_attr(not(feature = "client"), expect(unused_macro_rules))]
macro_rules! convenience_props {
    (@prop
        $trait_name:ident
        $(#[doc = $doc:literal])+
        #[
            $(#[doc = $with_set_doc:literal])+
            set
        ]
        $prop:ident($($sub_prop:ident),+) : $ty:ty
    ) => {
        convenience_props!(@prop
            $trait_name
            $(#[doc = $doc])+
            $prop($($sub_prop),+) : $ty
        );

        paste::paste! {
            $(#[doc = $with_set_doc])+
            ///
            /// This is an aggregation of following methods, see their docs for more details:
            $(
                #[doc = " - [`set_" $sub_prop "`](" $trait_name "::set_" $sub_prop ")"]
            )+
            pub async fn [<set_ $prop>](&self, $prop: $ty) -> ASCOMResult<()> {
                let [$($sub_prop),+] = $crate::api::macros::ConvertConvenienceProp::into_arr($prop);
                tokio::try_join!(
                    $(self.[<set_ $sub_prop>]($sub_prop),)+
                ).map(|_| ())
            }
        }
    };

    (@prop
        $trait_name:ident
        $(#[doc = $doc:literal])+
        $prop:ident($($sub_prop:ident),+) : $ty:ty
    ) => {
        $(#[doc = $doc])+
        ///
        /// This is an aggregation of following methods, see their docs for more details:
        $(
            #[doc = concat!(" - [`", stringify!($sub_prop), "`](", stringify!($trait_name), "::", stringify!($sub_prop), ")")]
        )+
        pub async fn $prop(&self) -> ASCOMResult<$ty> {
            tokio::try_join!($(self.$sub_prop(),)+)
            .map(|tuple| $crate::api::macros::ConvertConvenienceProp::from_arr(tuple.into()))
        }
    };

    ($trait_name:ident { $(
        $(# $attr:tt)*
        $prop:ident($($sub_prop:ident),*) : $ty:ty,
    )* }) => {
        /// Convenience methods for the client to get/set related properties together.
        #[cfg(feature = "client")]
        impl dyn $trait_name {
            $(
                convenience_props!(@prop
                    $trait_name
                    $(# $attr)*
                    $prop($($sub_prop),*) : $ty
                );
            )*
        }
    };
}

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

        /// ```no_run
        /// async fn setup(&self) -> eyre::Result<String>
        /// # { unimplemented!() }
        /// ```
        ///
        /// Web page user interface that enables device specific configuration to be set for each available device.
        ///
        /// The server should implement this to return HTML string. You can use [`Self::action`] to store the configuration.
        ///
        /// Note: on the client side you almost never want to just retrieve HTML to show it in the browser, as that breaks relative URLs.
        /// Use the `/{device_type}/{device_number}/setup` URL instead.
        fn setup(&self) -> futures::future::BoxFuture<'_, eyre::Result<String>> {
            Box::pin(futures::future::ok(include_str!("../server/device_setup_template.html").to_owned()))
        }
    };
    (@extras $trait_name:ident trait) => {
        /// Return all operational properties of this device.
        ///
        /// See [What is the “read all” feature and what are its rules?](https://ascom-standards.org/newdocs/readall-faq.html#readall-faq).
        fn device_state<'this: 'async_trait, 'async_trait>(&'this self) -> crate::api::ASCOMResultFuture<'async_trait, crate::api::TimestampedDeviceState<DeviceState>> {
            Box::pin(async move {
                Ok(crate::api::TimestampedDeviceState::new(DeviceState::new(self).await))
            })
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
    (@extras $trait_name:ident client) => {
        #[expect(single_use_lifetimes)] // we need compat with #[async_trait]
        fn device_state<'this: 'async_trait, 'async_trait>(&'this self) -> crate::api::ASCOMResultFuture<'async_trait, crate::api::TimestampedDeviceState<DeviceState>> {
            Box::pin(async move {
                match self.exec_action(Action::DeviceState).await.map(crate::api::device_state::de::TimestampedDeviceStateRepr::into_inner) {
                    Err(crate::ASCOMError { code: crate::ASCOMErrorCode::NOT_IMPLEMENTED, .. }) => {
                        // Fallback to individual property retrieval.
                        Ok(crate::api::TimestampedDeviceState::new(DeviceState::new(self).await))
                    }
                    result => result,
                }
            })
        }
    };

    (@extras Device mod) => {
        #[cfg(feature = "server")]
        #[derive(serde::Serialize)]
        pub(super) struct DeviceState; // dummy, we don't have any properties here

        #[cfg(feature = "server")]
        impl dyn Device {
            async fn device_state(&self) -> ASCOMResult<crate::api::TimestampedDeviceState<DeviceState>> {
                // we don't expose Device::device_state, but we do need to handle it on the server
                Ok(crate::api::TimestampedDeviceState::new(DeviceState))
            }
        }
    };
    (@extras $trait_name:ident mod) => {
        impl super::RetrieavableDevice for dyn $trait_name {
            const TYPE: super::DeviceType = super::DeviceType::$trait_name;

            fn get_storage(storage: &super::Devices) -> &[std::sync::Arc<Self>] {
                &storage.$trait_name
            }
        }

        impl super::RegistrableDevice<dyn $trait_name> for std::sync::Arc<dyn $trait_name> {
            fn add_to(self, storage: &mut super::Devices) {
                storage.$trait_name.push(self);
            }
        }

        impl<T: 'static + $trait_name> super::RegistrableDevice<dyn $trait_name> for T {
            fn add_to(self, storage: &mut super::Devices) {
                storage.$trait_name.push(std::sync::Arc::new(self));
            }
        }

        #[cfg(test)]
        #[tokio::test]
        async fn run_proxy_tests() -> eyre::Result<()> {
            $crate::test::run_proxy_tests::<dyn $trait_name>().await
        }
    };

    // We only have dummy device state in the server-side handler.
    (@device_state Device) => {};

    // Switch needs some special handling to gather device state across all devices.
    (@device_state Switch) => {};

    (@device_state $trait_name:ident $(
        $name:ident : $wire_name:ident as $ty:ty
    )*) => {
        /// An object representing all operational properties of the device.
        #[derive(Default, Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
        #[serde(rename_all = "PascalCase")]
        #[allow(clippy::unsafe_derive_deserialize)] // seems to be false positive
        pub struct DeviceState {
            $(
                #[doc = concat!("Result of [`", stringify!($trait_name), "::", stringify!($name), "`].")]
                #[serde(skip_serializing_if = "Option::is_none")]
                pub $name: Option<$ty>,
            )*
        }

        impl DeviceState {
            async fn new(_device: &(impl ?Sized + $trait_name)) -> Self {
                let ($($name,)*) = futures::join!($(_device.$name(),)*);

                Self {
                    $($name: $name.ok(),)*
                }
            }
        }
    };

    (@via $return_type:ty, $via:ty) => ($via);
    (@via $return_type:ty) => ($return_type);

    (@body ; $($header:tt)*) => {
        $($header)* ;
    };
    (@body $default_body:block $($header:tt)*) => {
        $($header)* {
            Box::pin(async move $default_body)
        }
    };

    (
        $(# $attr:tt)*
        $pub:vis trait $trait_name:ident: $trait_parents:ty {
            $(
                $(#[doc = $doc:literal])*
                #[http($method_path:literal, method = $http_method:ident $(, via = $via:ty)? $(, device_state = $device_state_name:ident)?)]
                $(# $method_attr:tt)*
                async fn $method_name:ident(
                    & $self:ident $(, #[http($param_query:literal $(, via = $param_via:ty)?)] $param:ident: $param_ty:ty)* $(,)?
                ) -> ASCOMResult<$return_type:ty> $default_body:tt
            )*
        }
    ) => (paste::paste! {
        rpc_trait!(@device_state $trait_name $($(
            $method_name: $device_state_name as $return_type
        )?)*);

        #[cfg_attr(feature = "client", derive(serde::Serialize), serde(untagged))]
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
            DeviceState,
        }

        #[cfg(feature = "server")]
        #[derive(serde::Serialize)]
        #[serde(untagged)]
        #[expect(non_camel_case_types)]
        pub(super) enum Response {
            $(
                $method_name(rpc_trait!(@via $return_type $(, $via)?)),
            )*
            #[serde(with = "crate::api::device_state::ser")]
            DeviceState(crate::api::TimestampedDeviceState<DeviceState>),
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
                    ("devicestate", _) => Self::DeviceState,
                    _ => return Ok(None),
                }))
            }

            #[cfg(feature = "client")]
            fn into_parts(self) -> $crate::params::ActionParams<impl serde::Serialize> {
                let (method, action) = match self {
                    $(Self::$method_name { .. } => ($crate::params::Method::$http_method, $method_path),)*
                    Self::DeviceState => ($crate::params::Method::Get, "devicestate"),
                };

                $crate::params::ActionParams {
                    action,
                    method,
                    params: self,
                }
            }
        }

        $(# $attr)*
        #[allow(unused_variables)]
        #[expect(single_use_lifetimes)]
        $pub trait $trait_name: $trait_parents {
            $(rpc_trait!(@body $default_body
                /// ```no_run
                #[doc = concat!("async fn ", stringify!($method_name), "(&self", $(", ", stringify!($param), ": ", stringify!($param_ty),)* ") -> ASCOMResult<", stringify!($return_type), ">")]
                /// # { unimplemented!() }
                /// ```
                ///
                $(#[doc = $doc])*
                $(# $method_attr)*
                fn $method_name<'this: 'async_trait, 'async_trait>(
                    &'this $self $(, $param: $param_ty)*
                ) -> crate::api::ASCOMResultFuture<'async_trait, $return_type>
            );)*

            rpc_trait!(@extras $trait_name trait);
        }

        #[cfg(feature = "client")]
        #[async_trait::async_trait]
        #[allow(useless_deprecated)]
        impl $trait_name for $crate::client::RawDeviceClient {
            $(
                $(# $method_attr)*
                async fn $method_name(
                    & $self $(, $param: $param_ty)*
                ) -> ASCOMResult<$return_type> {
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
        #[allow(deprecated)]
        impl Action {
            pub(super) async fn handle(self, device: std::sync::Arc<dyn $trait_name>) -> ASCOMResult<Response> {
                match self {
                    $(
                        Self::$method_name { $($param),* } => {
                            device.$method_name($($param),*)
                            .await
                            $(.map(<$via>::from))?
                            .map(Response::$method_name)
                        }
                    )*
                    Self::DeviceState => {
                        device.device_state()
                        .await
                        .map(Response::DeviceState)
                    }
                }
            }
        }

        rpc_trait!(@extras $trait_name mod);
    });
}

macro_rules! rpc_mod {
    ($(# $cfg:tt $trait_name:ident = $path:literal,)*) => (paste::paste! {
        $(
            # $cfg
            #[doc = "Types related to [`" $trait_name "`] devices."]
            pub mod [<$trait_name:snake>];

            # $cfg
            pub use [<$trait_name:snake>]::$trait_name;
        )*

        #[derive(PartialOrd, Ord, PartialEq, Eq, Clone, Copy, Debug, derive_more::Display, serde::Serialize, serde::Deserialize)]
        pub(crate) enum DeviceType {
            $(
                # $cfg
                #[display($path)]
                $trait_name,
            )*
        }

        /// A tagged enum wrapper for a type-erased instance of a device.
        #[derive(Clone, Debug)]
        #[expect(missing_docs)] // self-explanatory variants
        pub enum TypedDevice {
            $(
                # $cfg
                $trait_name(std::sync::Arc<dyn $trait_name>),
            )*
        }

        impl RegistrableDevice<dyn Device> for TypedDevice {
            fn add_to(self, storage: &mut Devices) {
                match self {
                    $(
                        # $cfg
                        Self::$trait_name(device) => storage.$trait_name.push(device),
                    )*
                }
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
                # $cfg
                $trait_name: Vec<std::sync::Arc<dyn $trait_name>>,
            )*
        }

        impl std::fmt::Debug for Devices {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let mut f = f.debug_struct("Devices");
                $(
                    # $cfg
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
                        # $cfg
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
            pub fn iter_all(&self) -> impl Iterator<Item = (TypedDevice, usize)> {
                let iter = std::iter::empty();

                $(
                    # $cfg
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

        #[cfg(feature = "client")]
        impl $crate::client::RawDeviceClient {
            pub(crate) const fn into_typed_client(self: std::sync::Arc<Self>, device_type: DeviceType) -> TypedDevice {
                match device_type {
                    $(
                        # $cfg
                        DeviceType::$trait_name => TypedDevice::$trait_name(self),
                    )*
                }
            }
        }

        #[cfg(feature = "server")]
        #[derive(serde::Deserialize)]
        #[serde(remote = "DeviceType")]
        pub(crate) enum DevicePath {
            $(
                # $cfg
                #[serde(rename = $path)]
                $trait_name,
            )*
        }

        #[cfg(feature = "server")]
        const _: () = {
            impl TypedDevice {
                pub(crate) fn to_configured_device(&self, as_number: usize) -> ConfiguredDevice<DeviceType> {
                    match *self {
                        $(
                            # $cfg
                            Self::$trait_name(ref device) => device.to_configured_device(as_number),
                        )*
                    }
                }
            }

            #[derive(serde::Serialize)]
            #[serde(untagged)]
            enum TypedResponse {
                Device(device::Response),
                $(
                    # $cfg
                    $trait_name([<$trait_name:snake>]::Response),
                )*
            }

            enum TypedDeviceAction {
                Device(device::Action),
                $(
                    # $cfg
                    $trait_name([<$trait_name:snake>]::Action),
                )*
            }

            impl TypedDeviceAction {
                fn from_parts(device_type: DeviceType, action: &str, mut params: crate::server::ActionParams) -> crate::server::Result<Self> {
                    let result = match device_type {
                        $(
                            # $cfg
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

            impl Devices {
                pub(crate) fn get_device_for_server(
                    &self,
                    device_type: DeviceType,
                    device_number: usize,
                ) -> $crate::server::Result<Arc<dyn Device>> {
                    // With trait upcasting, we can get any device as dyn Device directly
                    Ok(match device_type {
                        $(
                            # $cfg
                            DeviceType::$trait_name => {
                                self.get_for_server::<dyn $trait_name>(device_number)?
                            }
                        )*
                    })
                }

                pub(crate) async fn handle_action<'this>(&'this self, device_type: DeviceType, device_number: usize, action: &'this str, params: $crate::server::ActionParams) -> $crate::server::Result<impl serde::Serialize + use<>> {
                    let action = TypedDeviceAction::from_parts(device_type, action, params)?;

                    Ok(match action {
                        $(
                            # $cfg
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
        };
    });
}
