use crate::transaction::ASCOMRequest;
use crate::Devices;
use axum::extract::Path;
use axum::http::{Method, StatusCode};
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
                        let mut device =
                            self.get(&device_type, device_number)
                            .ok_or((StatusCode::NOT_FOUND, "Device not found"))?
                            .lock()
                            .map_err(|_err| (StatusCode::INTERNAL_SERVER_ERROR, "This device can't be accessed anymore due to a previous fatal error"))?;

                        Ok::<_, axum::response::ErrorResponse>(Json(request.respond_with(move |params| {
                            device.handle_action(method == Method::PUT, &action, params)
                        })))
                },
            ),
        )
    }
}
