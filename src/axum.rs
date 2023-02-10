use crate::api::{Camera, ImageArrayResponse};
use crate::transaction::ASCOMRequest;
use crate::{Devices, OpaqueResponse};
use axum::extract::Path;
use axum::http::header::CONTENT_TYPE;
use axum::http::Method;
use axum::response::IntoResponse;
use axum::routing::{on, MethodFilter};
use axum::{Form, Router, TypedHeader};
use mediatype::MediaTypeList;
use serde::Serialize;
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
    pub fn into_router(self) -> Router {
        let this = Arc::new(self);

        Router::new()
            .route(
                "/management/apiversions",
                axum::routing::get(|Form(request): Form<ASCOMRequest>| {
                    let span = request.transaction.span();

                    async move {
                        request
                            .transaction
                            .make_response(OpaqueResponse::try_from([1_u32]))
                    }
                    .instrument(span)
                }),
            )
            .route("/management/v1/configureddevices", {
                let this = Arc::clone(&this);

                axum::routing::get(|Form(request): Form<ASCOMRequest>| {
                    let span = request.transaction.span();

                    async move {
                        #[derive(Serialize)]
                        struct IterSerialize<I: Iterator + Clone>(
                            #[serde(with = "serde_iter::seq")] I,
                        )
                        where
                            I::Item: Serialize;

                        request
                            .transaction
                            .make_response(OpaqueResponse::try_from(IterSerialize(this.iter())))
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
                          Form(request): Form<ASCOMRequest>| {
                        let span = request.transaction.span();

                        async move {
                            if accepts_image_bytes.accepts
                                && method == Method::GET
                                && device_type == "camera"
                                && action == "imagearray"
                            {
                                return <dyn Camera>::with(&this, device_number, |device| {
                                    (
                                        [(CONTENT_TYPE, "application/imagebytes")],
                                        ImageArrayResponse::to_image_bytes(
                                            &device.image_array(),
                                            &request.transaction,
                                        ),
                                    )
                                })
                                .into_response();
                            }

                            this.handle_action(
                                &device_type,
                                device_number,
                                method == Method::PUT,
                                &action,
                                request.encoded_params,
                            )
                            .map(|result| request.transaction.make_response(result))
                            .into_response()
                        }
                        .instrument(span)
                    },
                ),
            )
    }
}
