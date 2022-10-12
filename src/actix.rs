use crate::{respond_with, Devices};
use actix_web::error::ErrorBadRequest;
use actix_web::web::{Json, Path};
use actix_web::HttpRequest;
use serde::Serialize;

impl actix_web::dev::HttpServiceFactory for Devices {
    fn register(self, config: &mut actix_web::dev::AppService) {
        fn handler(
            request: &HttpRequest,
            path: Path<(String, usize, String)>,
            params: &str,
        ) -> actix_utils::future::Ready<actix_web::Result<Json<impl Serialize>>> {
            let devices = request
                .app_data::<Devices>()
                .expect("Devices should be stored as an app-data by now")
                .clone();

            let res = respond_with(params, move |params| {
                let (device_type, device_number, action) = path.into_inner();
                devices.handle_action(false, &device_type, device_number, &action, params)
            });

            actix_utils::future::ready(match res {
                Ok(body) => Ok(Json(body)),
                Err(err) => Err(ErrorBadRequest(err)),
            })
        }

        let resource = actix_web::web::resource("/api/v1/{device_type}/{device_number}/{action}")
            .app_data(self)
            .route(actix_web::web::get().to(
                move |request: HttpRequest, path: Path<(String, usize, String)>| {
                    handler(&request, path, request.query_string())
                },
            ))
            .route(actix_web::web::post().to(
                move |request: HttpRequest, path: Path<(String, usize, String)>, body: String| {
                    handler(&request, path, &body)
                },
            ));

        actix_web::dev::HttpServiceFactory::register(resource, config);
    }
}
