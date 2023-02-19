use crate::api::{Camera, ConfiguredDevice};
use crate::params::OpaqueParams;
use crate::response::OpaqueResponse;
use crate::transaction::server_handler;
use crate::Devices;
use axum::extract::Path;
use axum::http::Method;

use axum::routing::{on, MethodFilter};
use axum::{Form, Router, TypedHeader};
use futures::StreamExt;
use mediatype::MediaTypeList;
use std::sync::Arc;

// A hack until TypedHeader supports Accept natively.
struct AcceptsImageBytes {
    accepts: bool,
}

impl axum::headers::Header for AcceptsImageBytes {
    fn name() -> &'static axum::headers::HeaderName {
        static ACCEPT: axum::headers::HeaderName = axum::headers::HeaderName::from_static("accept");
        &ACCEPT
    }

    fn decode<'value, I>(values: &mut I) -> Result<Self, axum::headers::Error>
    where
        Self: Sized,
        I: Iterator<Item = &'value axum::http::HeaderValue>,
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

    fn encode<E: Extend<axum::http::HeaderValue>>(&self, values: &mut E) {
        values.extend(std::iter::once(axum::http::HeaderValue::from_static(
            if self.accepts {
                "application/imagebytes"
            } else {
                "*/*"
            },
        )));
    }
}

impl Devices {
    pub fn into_router(self) -> Router {
        let this = Arc::new(self);

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
            .route(
                "/api/v1/:device_type/:device_number/:action",
                on(
                    MethodFilter::GET | MethodFilter::PUT,
                    move |method: Method,
                          Path((device_type, device_number, action)): Path<(
                        String,
                        usize,
                        String,
                    )>,
                          TypedHeader(accepts_image_bytes): TypedHeader<AcceptsImageBytes>,
                          Form(params): Form<OpaqueParams>| {
                        let is_mut = method == Method::PUT;

                        async move {
                            if accepts_image_bytes.accepts
                                && method == Method::GET
                                && device_type == "camera"
                                && action == "imagearray"
                            {
                                return server_handler(&format!("/api/v1/{device_type}/{device_number}/{action} with ImageBytes"), is_mut, params, |params| async move {
                                    params.finish_extraction();

                                    match <dyn Camera>::get_in(&this, device_number).await {
                                        Some(device) => Ok(device.image_array().await),
                                        None => Err((
                                            axum::http::StatusCode::NOT_FOUND,
                                            "Device not found",
                                        ).into()),
                                    }
                            }).await;
                        }

                            server_handler(&format!("/api/v1/{device_type}/{device_number}/{action}"), is_mut, params, |params| {
                                this.handle_action(
                                    &device_type,
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
