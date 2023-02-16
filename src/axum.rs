use crate::api::{Camera, ConfiguredDevice, ImageArrayResponse};
use crate::rpc::Sender;
use crate::transaction::ASCOMRequest;
use crate::{Devices, OpaqueResponse};
use axum::extract::Path;
use axum::http::header::CONTENT_TYPE;
use axum::http::Method;
use axum::response::{ErrorResponse, IntoResponse};
use axum::routing::{on, MethodFilter};
use axum::{Router, TypedHeader};
use futures::StreamExt;
use mediatype::MediaTypeList;
use reqwest::IntoUrl;
use std::sync::Arc;
use tracing::Instrument;

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
    pub async fn from_server(client: reqwest::Client, url: impl IntoUrl) -> anyhow::Result<Self> {
        let mut devices = Self::default();
        let url = Arc::new(url.into_url()?);
        client
            .get(url.join("management/v1/configureddevices")?)
            .send()
            .await?
            // TODO: handle $.Value
            .json::<Vec<ConfiguredDevice>>()
            .await?
            .into_iter()
            .try_for_each(|device| {
                let sender = Sender {
                    client: client.clone(),
                    base: url.clone(),
                    unique_id: device.unique_id,
                    device_number: device.device_number,
                };
                sender.add_as(&device.device_type, &mut devices)
            })?;
        Ok(devices)
    }

    pub fn into_router(self) -> Router {
        let this = Arc::new(self);

        Router::new()
            .route(
                "/management/apiversions",
                axum::routing::get(|request: ASCOMRequest| {
                    let span = request.transaction.span();

                    async move {
                        Ok::<_, ErrorResponse>(
                            request
                                .transaction
                                .make_response(OpaqueResponse::new([1_u32])),
                        )
                    }
                    .instrument(span)
                }),
            )
            .route("/management/v1/configureddevices", {
                let this = Arc::clone(&this);

                axum::routing::get(|request: ASCOMRequest| {
                    let span = request.transaction.span();

                    async move {
                        let devices = this.iter().collect::<Vec<ConfiguredDevice>>().await;

                        Ok::<_, ErrorResponse>(
                            request
                                .transaction
                                .make_response(OpaqueResponse::new(devices)),
                        )
                    }
                    .instrument(span)
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
                          request: ASCOMRequest| {
                        let span = request.transaction.span();

                        async move {
                            if accepts_image_bytes.accepts
                                && method == Method::GET
                                && device_type == "camera"
                                && action == "imagearray"
                            {
                                match <dyn Camera>::get_in(&this, device_number).await {
                                    Some(device) => Ok((
                                        [(CONTENT_TYPE, "application/imagebytes")],
                                        ImageArrayResponse::to_image_bytes(
                                            &device.image_array().await,
                                            &request.transaction,
                                        ),
                                    )
                                        .into_response()),
                                    None => Err(ErrorResponse::from((
                                        axum::http::StatusCode::NOT_FOUND,
                                        "Device not found",
                                    ))),
                                }
                            } else {
                                Ok(request
                                    .transaction
                                    .make_response(
                                        this.handle_action(
                                            &device_type,
                                            device_number,
                                            method == Method::PUT,
                                            &action,
                                            request.encoded_params,
                                        )
                                        .await?,
                                    )
                                    .into_response())
                            }
                        }
                        .instrument(span)
                    },
                ),
            )
    }
}
