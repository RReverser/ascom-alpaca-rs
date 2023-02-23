use crate::api::{Camera, ConfiguredDevice, DevicePath, DeviceType, ServerInfo};
use crate::params::OpaqueParams;
use crate::response::OpaqueResponse;
use crate::transaction::server_handler;
use crate::Devices;
use async_trait::async_trait;
use axum::extract::{FromRequest, Path};
use axum::headers::{Header, HeaderName, HeaderValue};
use axum::http::Method;
use axum::routing::{on, MethodFilter};
use axum::{Form, Router};
use futures::StreamExt;
use mediatype::MediaTypeList;
use serde::Serialize;
use std::sync::Arc;

// A hack until TypedHeader supports Accept natively.
#[derive(Default)]
struct AcceptsImageBytes {
    accepts: bool,
}

#[async_trait]
impl<B: Send> FromRequest<B> for AcceptsImageBytes {
    type Rejection = std::convert::Infallible;

    async fn from_request(
        req: &mut axum::extract::RequestParts<B>,
    ) -> Result<Self, Self::Rejection> {
        Ok(Self::decode(&mut req.headers().get_all("accept").into_iter()).unwrap_or_default())
    }
}

impl Header for AcceptsImageBytes {
    fn name() -> &'static HeaderName {
        static ACCEPT: HeaderName = HeaderName::from_static("accept");
        &ACCEPT
    }

    fn decode<'value, I>(values: &mut I) -> Result<Self, axum::headers::Error>
    where
        Self: Sized,
        I: Iterator<Item = &'value HeaderValue>,
    {
        let mut accepts = false;
        for value in values {
            for media_type in MediaTypeList::new(
                value
                    .to_str()
                    .map_err(|_err| axum::headers::Error::invalid())?,
            ) {
                let media_type = media_type.map_err(|_err| axum::headers::Error::invalid())?;
                if media_type.ty == mediatype::names::APPLICATION
                    && media_type.subty == "imagebytes"
                    && media_type.suffix.is_none()
                {
                    accepts = true;
                    break;
                }
            }
        }
        Ok(Self { accepts })
    }

    fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
        values.extend(std::iter::once(HeaderValue::from_static(if self.accepts {
            "application/imagebytes"
        } else {
            "*/*"
        })));
    }
}

#[derive(Debug, Serialize)]
struct ServerInfoValue {
    #[serde(rename = "Value")]
    server_info: ServerInfo,
}

impl Devices {
    pub fn into_router(self, server_info: ServerInfo) -> Router {
        let this = Arc::new(self);
        let server_info = OpaqueResponse::new(ServerInfoValue { server_info });

        Router::new()
            .route(
                "/management/apiversions",
                axum::routing::get(|Form(params): Form<OpaqueParams>| {
                    server_handler("/management/apiversions", false, params, |_params| async move {
                        Ok(OpaqueResponse::new([1_u32]))
                    })
                }),
            )
            .route("/management/v1/configureddevices", {
                let this = Arc::clone(&this);

                axum::routing::get(|Form(params): Form<OpaqueParams>| {
                    server_handler("/management/v1/configureddevices", false, params, |_params| async move {
                        let devices = this.stream_configured().collect::<Vec<ConfiguredDevice>>().await;
                        Ok(OpaqueResponse::new(devices))
                    })
                })
            })
            .route("/management/v1/description",
                axum::routing::get(move |Form(params): Form<OpaqueParams>| {
                    server_handler("/management/v1/serverinfo", false, params, |_params| async move {
                        Ok(server_info.clone())
                    })
                })
            )
            .route(
                "/api/v1/:device_type/:device_number/:action",
                on(
                    MethodFilter::GET | MethodFilter::PUT,
                    move |method: Method,
                          Path((DevicePath(device_type), device_number, mut action)): Path<(
                        DevicePath,
                        usize,
                        String,
                    )>,
                          accepts_image_bytes: AcceptsImageBytes,
                          Form(params): Form<OpaqueParams>| {
                        let is_mut = method == Method::PUT;

                        // imagearrayvariant is soft-deprecated; we should accept it but
                        // forward to the imagearray handler instead.
                        if device_type == DeviceType::Camera && action == "imagearrayvariant" {
                            action.truncate("imagearray".len());
                        }

                        async move {
                            if accepts_image_bytes.accepts
                                && method == Method::GET
                                && device_type == DeviceType::Camera
                                && action == "imagearray"
                            {
                                return server_handler(&format!("/api/v1/{device_type}/{device_number}/{action} with ImageBytes"), is_mut, params, |params| async move {
                                    params.finish_extraction();

                                    Ok(<dyn Camera>::get_in(&this, device_number).await?.image_array().await)
                            }).await;
                        }

                            server_handler(&format!("/api/v1/{device_type}/{device_number}/{action}"), is_mut, params, |params| {
                                this.handle_action(
                                    device_type,
                                    device_number,
                                    method == Method::PUT,
                                    &action,
                                    params,
                                )
                            }).await
                        }
                    }),
            )
    }
}
