use crate::api::{Camera, ConfiguredDevice, DevicePath, DeviceType, ServerInfo};
use crate::params::RawActionParams;
use crate::response::OpaqueResponse;
use crate::transaction::server_handler;
use crate::Devices;
use axum::extract::Path;
use axum::headers::{Header, HeaderName, HeaderValue};
use axum::routing::MethodFilter;
use axum::{Router, TypedHeader};
use futures::StreamExt;
use mediatype::MediaTypeList;
use serde::Serialize;
use std::sync::Arc;

// A hack until TypedHeader supports Accept natively.
#[derive(Default)]
struct AcceptsImageBytes {
    accepts: bool,
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
                axum::routing::get(|params: RawActionParams| {
                    server_handler("/management/apiversions",  params, |_params| async move {
                        Ok(OpaqueResponse::new([1_u32]))
                    })
                }),
            )
            .route("/management/v1/configureddevices", {
                let this = Arc::clone(&this);

                axum::routing::get(|params: RawActionParams| {
                    server_handler("/management/v1/configureddevices",  params, |_params| async move {
                        let devices = this.stream_configured().collect::<Vec<ConfiguredDevice>>().await;
                        Ok(OpaqueResponse::new(devices))
                    })
                })
            })
            .route("/management/v1/description",
                axum::routing::get(move |params: RawActionParams| {
                    server_handler("/management/v1/serverinfo", params, |_params| async move {
                        Ok(server_info.clone())
                    })
                })
            )
            .route(
                "/api/v1/:device_type/:device_number/:action",
                axum::routing::on(
                    MethodFilter::GET | MethodFilter::PUT,
                    move |
                          Path((DevicePath(device_type), device_number, mut action)): Path<(
                        DevicePath,
                        usize,
                        String,
                    )>,
                          accepts_image_bytes: TypedHeader<AcceptsImageBytes>,
                          params: RawActionParams| {
                        // imagearrayvariant is soft-deprecated; we should accept it but
                        // forward to the imagearray handler instead.
                        if device_type == DeviceType::Camera && action == "imagearrayvariant" {
                            action.truncate("imagearray".len());
                        }

                        async move {
                            if accepts_image_bytes.accepts
                                && matches!(params, RawActionParams::Get { .. })
                                && device_type == DeviceType::Camera
                                && action == "imagearray"
                            {
                                return server_handler(&format!("/api/v1/{device_type}/{device_number}/{action} with ImageBytes"), params, |_params| async move {
                                    Ok(<dyn Camera>::get_in(&this, device_number)?.read().await.image_array().await)
                                }).await;
                            }

                            server_handler(&format!("/api/v1/{device_type}/{device_number}/{action}"),  params, |params| {
                                this.handle_action(
                                    device_type,
                                    device_number,
                                    &action,
                                    params,
                                )
                            }).await
                        }
                    }),
            )
    }
}
