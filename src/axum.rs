use crate::transaction::ASCOMRequest;
use crate::Devices;
use axum::extract::Path;
use axum::http::Method;
use axum::routing::{on, MethodFilter};
use axum::{Form, Json, Router};

impl Devices {
    pub fn into_router(self) -> Router {
        Router::new().route(
            "/api/v1/:device_type/:device_number/:action",
            on(
                MethodFilter::GET | MethodFilter::PUT,
                move |method: Method,
                      Path((device_type, device_number, action)): Path<(String, usize, String)>,
                      Form(request): Form<ASCOMRequest>| async move {
                    Json(request.respond_with(move |params| {
                        self.handle_action(
                            method == Method::PUT,
                            &device_type,
                            device_number,
                            &action,
                            params,
                        )
                    }))
                },
            ),
        )
    }
}
