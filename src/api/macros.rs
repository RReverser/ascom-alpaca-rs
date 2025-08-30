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

    // Don't add any extra code for other traits in other locations.
    (@extras $trait_name:ident $loc:ident) => {};

    (@device_state $trait_name:ident $(
        $name:ident : $wire_name:ident as $ty:ty
    )*) => {
        /// An object representing all operational properties of the device.
        #[derive(Default, Debug, Clone, Copy)]
        pub struct DeviceState {
            /// The time at which the operational states were measured, when known.
            pub timestamp: Option<std::time::SystemTime>,
            $(
                #[doc = concat!("Result of [`", stringify!($trait_name), "::", stringify!($name), "`].")]
                pub $name: Option<$ty>,
            )*
        }

        impl DeviceState {
            async fn gather(_device: &(impl ?Sized + $trait_name)) -> Self {
                DeviceState {
                    timestamp: Some(std::time::SystemTime::now()),
                    $($name: _device.$name().await.ok(),)*
                }
            }
        }

        #[cfg(feature = "server")]
        impl serde::Serialize for DeviceState {
            fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                #[derive(serde::Serialize)]
                #[serde(rename_all = "PascalCase")]
                struct DeviceStateItem<T> {
                    name: &'static str,
                    value: T,
                }

                let mut seq = serializer.serialize_seq(None)?;
                if let Some(timestamp) = &self.timestamp {
                    serde::ser::SerializeSeq::serialize_element(&mut seq, &DeviceStateItem {
                        name: "TimeStamp",
                        value: crate::api::time_repr::TimeRepr::<crate::api::time_repr::Iso8601>::from(*timestamp),
                    })?;
                }
                $(if let Some(value) = &self.$name {
                    serde::ser::SerializeSeq::serialize_element(&mut seq, &DeviceStateItem {
                        name: stringify!($wire_name),
                        value,
                    })?;
                })*
                serde::ser::SerializeSeq::end(seq)
            }
        }

        #[cfg(feature = "client")]
        impl<'de> serde::Deserialize<'de> for DeviceState {
            fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                #[derive(serde::Deserialize)]
                #[serde(tag = "Name", content = "Value")]
                enum DeviceStateItem {
                    TimeStamp(crate::api::time_repr::TimeRepr::<crate::api::time_repr::Iso8601>),
                    $($wire_name($ty),)*
                }

                struct Visitor;

                impl<'de> serde::de::Visitor<'de> for Visitor {
                    type Value = DeviceState;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        formatter.write_str("a sequence of DeviceStateItem objects")
                    }

                    fn visit_seq<A: serde::de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                        let mut state = DeviceState {
                            timestamp: None,
                            $($name: None,)*
                        };

                        while let Some(item) = seq.next_element::<DeviceStateItem>()? {
                            match item {
                                DeviceStateItem::TimeStamp(value) => state.timestamp = Some(value.into()),
                                $(DeviceStateItem::$wire_name(value) => state.$name = Some(value),)*
                            }
                        }

                        Ok(state)
                    }
                }

                deserializer.deserialize_seq(Visitor)
            }
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
                $method_name($return_type),
            )*
            DeviceState(DeviceState),
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
        #[async_trait::async_trait]
        #[allow(unused_variables)]
        $pub trait $trait_name: $trait_parents {
            $(
                $(#[doc = $doc])*
                ///
                /// Definition before the `#[async_trait]` expansion:
                ///
                /// ```ignore
                #[doc = concat!("async fn ", stringify!($method_name), "(&self", $(", ", stringify!($param), ": ", stringify!($param_ty),)* ") -> ASCOMResult<", stringify!($return_type), ">")]
                /// # { unimplemented!() }
                /// ```
                $(# $method_attr)*
                async fn $method_name(
                    & $self $(, $param: $param_ty)*
                ) -> ASCOMResult<$return_type> $default_body
            )*

            rpc_trait!(@extras $trait_name trait);

            /// Return all operational properties of this device.
            ///
            /// See [What is the “read all” feature and what are its rules?](https://ascom-standards.org/newdocs/readall-faq.html#readall-faq).
            async fn device_state(&self) -> ASCOMResult<DeviceState> {
                Ok(DeviceState::gather(self).await)
            }
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
                            device.$method_name($($param),*).await.map(Response::$method_name)
                        }
                    )*
                    Self::DeviceState => {
                        $trait_name::device_state(&*device).await.map(Response::DeviceState)
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
            pub fn iter_all(&self) -> impl '_ + Iterator<Item = (TypedDevice, usize)> {
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

                pub(crate) async fn handle_action<'this>(&'this self, device_type: DeviceType, device_number: usize, action: &'this str, params: $crate::server::ActionParams) -> $crate::server::Result<impl serde::Serialize> {
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
