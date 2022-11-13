use crate::{ASCOMError, ASCOMErrorCode, ASCOMResult};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct OpaqueResponse(serde_json::Map<String, serde_json::Value>);

impl OpaqueResponse {
    pub(crate) fn try_from<T: Serialize>(value: T) -> ASCOMResult<Self> {
        let json = serde_json::to_value(value)
            .map_err(|err| ASCOMError::new(ASCOMErrorCode::INVALID_VALUE, err.to_string()))?;

        Ok(Self(match json {
            serde_json::Value::Object(map) => map,
            serde_json::Value::Null => serde_json::Map::new(),
            value => {
                // Wrap into IntResponse / BoolResponse / ..., aka {"value": ...}
                std::iter::once(("Value".to_owned(), value)).collect()
            }
        }))
    }
}

macro_rules! rpc {
    (@if_parent $parent_trait_name:ident { $($then:tt)* } { $($else:tt)* }) => {
        $($then)*
    };

    (@if_parent { $($then:tt)* } { $($else:tt)* }) => {
        $($else)*
    };

    (@is_mut mut self) => (true);

    (@is_mut self) => (false);

    ($(
        $(#[doc = $doc:literal])*
        #[http($path:literal)]
        pub trait $trait_name:ident $(: $parent_trait_name:ident)? {
            $(
                $(#[doc = $method_doc:literal])*
                #[http($method_path:literal $(, $params_ty:ty)?)]
                fn $method_name:ident(& $($mut_self:ident)* $(, $param:ident: $param_ty:ty)* $(,)?) $(-> $return_type:ty)?;
            )*
        }
    )*) => {
        $(
            #[allow(unused_variables)]
            $(#[doc = $doc])*
            pub trait $trait_name $(: $parent_trait_name)? {
                rpc!(@if_parent $($parent_trait_name)? {} {
                    fn ty(&self) -> &'static str;

                    fn handle_action(&mut self, is_mut: bool, action: &str, params: $crate::transaction::ASCOMParams) -> $crate::ASCOMResult<$crate::OpaqueResponse>;
                });

                $(
                    $(#[doc = $method_doc])*
                    fn $method_name(& $($mut_self)* $(, $param: $param_ty)*) -> $crate::ASCOMResult$(<$return_type>)? {
                        Err($crate::ASCOMError::NOT_IMPLEMENTED)
                    }
                )*
            }

            // Split this out from the trait because it's not meant to be overridden,
            // or, for that matter, used outside of ascom-alpaca-dslr itself.
            impl dyn $trait_name {
                pub const TYPE: &'static str = $path;

                pub fn handle_action_impl<T: $trait_name>(device: &mut T, is_mut: bool, action: &str, params: $crate::transaction::ASCOMParams) -> $crate::ASCOMResult<$crate::OpaqueResponse> {
                    match (is_mut, action) {
                        $((rpc!(@is_mut $($mut_self)*), $method_path) => {
                            let span = tracing::info_span!(concat!(stringify!($trait_name), "::", stringify!($method_name)));
                            let _enter = span.enter();

                            $(
                                let params: $params_ty =
                                    params.try_as()
                                    .map_err(|err| {
                                        tracing::error!(raw_params = ?params, ?err, "Could not decode params");
                                        $crate::ASCOMError::new($crate::ASCOMErrorCode::INVALID_VALUE, err.to_string())
                                    })?;
                            )?
                            tracing::info!($($param = ?&params.$param,)* "Calling Alpaca handler");
                            let result = device.$method_name($(params.$param),*)?;
                            tracing::debug!(?result, "Alpaca handler returned");
                            $crate::OpaqueResponse::try_from(result)
                        })*
                        _ => {
                            rpc!(@if_parent $($parent_trait_name)? {
                                <dyn $($parent_trait_name)?>::handle_action_impl(device, is_mut, action, params)
                            } {
                                Err($crate::ASCOMError::NOT_IMPLEMENTED)
                            })
                        }
                    }
                }
            }
        )*
    };
}

pub(crate) use rpc;
