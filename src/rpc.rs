use crate::{ASCOMError, ASCOMErrorCode, ASCOMParams, ASCOMResult};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct OpaqueResponse(pub(crate) serde_json::Map<String, serde_json::Value>);

impl OpaqueResponse {
    pub(crate) fn new<T: Debug + Serialize>(value: T) -> Self {
        let json = serde_json::to_value(&value).unwrap_or_else(|err| {
            // This should never happen, but if it does, log and return the error.
            // This simplifies error handling for this rare case without having to panic!.
            tracing::error!(?value, %err, "Serialization failure");
            serde_json::to_value(ASCOMError {
                code: ASCOMErrorCode::UNSPECIFIED,
                message: format!("Failed to serialize {value:#?}: {err}").into(),
            })
            .expect("ASCOMError can never fail to serialize")
        });

        Self(match json {
            serde_json::Value::Object(map) => map,
            serde_json::Value::Null => serde_json::Map::new(),
            value => {
                // Wrap into IntResponse / BoolResponse / ..., aka {"value": ...}
                std::iter::once(("Value".to_owned(), value)).collect()
            }
        })
    }
}

pub(crate) trait ASCOMParam: Sized {
    fn from_string(s: String) -> anyhow::Result<Self>;
    fn to_string(self) -> String;
}

impl ASCOMParam for String {
    fn from_string(s: String) -> anyhow::Result<Self> {
        Ok(s)
    }

    fn to_string(self) -> String {
        self
    }
}

impl ASCOMParam for bool {
    fn from_string(s: String) -> anyhow::Result<Self> {
        Ok(match s.as_str() {
            "True" => true,
            "False" => false,
            _ => anyhow::bail!(r#"Invalid bool value {s:?}, expected "True" or "False""#),
        })
    }

    fn to_string(self) -> String {
        match self {
            true => "True",
            false => "False",
        }
        .to_owned()
    }
}

macro_rules! simple_ascom_param {
    ($($ty:ty),*) => {
        $(
            impl ASCOMParam for $ty {
                fn from_string(s: String) -> anyhow::Result<Self> {
                    Ok(s.parse()?)
                }

                fn to_string(self) -> String {
                    ToString::to_string(&self)
                }
            }
        )*
    };
}

simple_ascom_param!(i32, u32, f64);

macro_rules! ascom_enum {
    ($name:ty) => {
        impl $crate::rpc::ASCOMParam for $name {
            fn from_string(s: String) -> anyhow::Result<Self> {
                Ok(<Self as num_enum::TryFromPrimitive>::try_from_primitive(
                    $crate::rpc::ASCOMParam::from_string(s)?,
                )?)
            }

            fn to_string(self) -> String {
                let primitive: <Self as num_enum::TryFromPrimitive>::Primitive = self.into();
                $crate::rpc::ASCOMParam::to_string(primitive)
            }
        }
    };
}
pub(crate) use ascom_enum;

#[derive(Debug)]
pub struct Sender {
    client: reqwest::Client,
    pub(crate) unique_id: String,
    base: reqwest::Url,
    device_number: usize,
}

impl Sender {
    pub(crate) async fn exec_action(
        &self,
        device_type: &str,
        is_mut: bool,
        action: &str,
        params: &ASCOMParams,
    ) -> ASCOMResult<OpaqueResponse> {
        let device_number = self.device_number;
        let request = self.client.request(
            if is_mut {
                reqwest::Method::PUT
            } else {
                reqwest::Method::GET
            },
            self.base
                .join(&format!("{device_type}/{device_number}/{action}"))
                .map_err(|err| {
                    tracing::error!("Could not construct an action URL: {}", err);
                    ASCOMError::new(ASCOMErrorCode::UNSPECIFIED, err.to_string())
                })?,
        );
        // TODO: add transaction IDs.
        let request = if is_mut {
            request.form(params)
        } else {
            request.query(params)
        };
        let res: OpaqueResponse = request
            .send()
            .await
            .and_then(reqwest::Response::error_for_status)
            .map_err(|err| {
                tracing::error!("HTTP error: {}", err);
                ASCOMError::new(ASCOMErrorCode::UNSPECIFIED, err.to_string())
            })?
            .json()
            .await
            .map_err(|err| {
                tracing::error!("Could not parse response: {}", err);
                ASCOMError::new(ASCOMErrorCode::UNSPECIFIED, err.to_string())
            })?;
        match res.0.get("ErrorNumber") {
            None => Ok(res),
            Some(value) if value == 0_i32 => Ok(res),
            _ => Err(
                ASCOMError::deserialize(serde_json::Value::from(res.0)).map_err(|err| {
                    tracing::error!("Could not deserialize error: {}", err);
                    ASCOMError::new(ASCOMErrorCode::UNSPECIFIED, err.to_string())
                })?,
            ),
        }
    }
}

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
            async fn handle_action(device: &mut (impl ?Sized + $trait_name), is_mut: bool, action: &str, #[allow(unused_mut)] mut params: $crate::transaction::ASCOMParams) -> axum::response::Result<$crate::OpaqueResponse> {
                use tracing::Instrument;

                match (is_mut, action) {
                    $(
                        (rpc!(@is_mut $($mut_self)*), $method_path) => async move {
                            $(
                                let $param = params.extract($param_query)?;
                            )*
                            tracing::debug!($(?$param,)* "Calling Alpaca handler");
                            Ok(match device.$method_name($($param),*).await {
                                Ok(value) => {
                                    tracing::debug!(?value, "Alpaca handler returned");
                                    $crate::OpaqueResponse::new(value)
                                },
                                Err(err) => {
                                    tracing::error!(%err, "Alpaca handler returned an error");
                                    $crate::OpaqueResponse::new(err)
                                },
                            })
                        }
                        .instrument(tracing::info_span!(concat!(stringify!($trait_name), "::", stringify!($method_name))))
                        .await
                        .map_err(|err: anyhow::Error| (axum::http::StatusCode::BAD_REQUEST, format!("{:#}", err)).into()),
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
        impl $trait_name for $crate::rpc::Sender {
            $($extra_impl_body)*

            $(
                async fn $method_name(& $($mut_self)* $(, $param: $param_ty)*) -> $crate::ASCOMResult$(<$return_type>)? {
                    use tracing::Instrument;

                    async move {
                        #[allow(unused_mut)]
                        let mut opaque_params = $crate::transaction::ASCOMParams::default();
                        $(
                            opaque_params.insert($param_query, $param);
                        )*
                        #[allow(unused_variables)]
                        let opaque_response = rpc!(@get_self $($mut_self)*).exec_action($path, rpc!(@is_mut $($mut_self)*), $method_path, &opaque_params).await?;
                        Ok({
                            $(
                                <$return_type as serde::Deserialize>::deserialize(serde_json::Value::from(opaque_response.0))
                                .map_err(|err| $crate::ASCOMError::new($crate::ASCOMErrorCode::UNSPECIFIED, err.to_string()))?
                            )?
                        })
                    }.instrument(tracing::info_span!(concat!(stringify!($trait_name), "::", stringify!($method_name)))).await
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
                    async fn unique_id(&self) -> String;
                } {
                    async fn unique_id(&self) -> String {
                        self.unique_id.clone()
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

            pub(crate) async fn handle_action(&self, device_type: &str, device_number: usize, is_mut: bool, action: &str, params: $crate::transaction::ASCOMParams) -> axum::response::Result<$crate::OpaqueResponse> {
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
