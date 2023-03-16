use crate::api::DeviceType;
use crate::params::OpaqueParams;
use crate::transaction::{Client, Response};
use crate::{ASCOMError, ASCOMErrorCode, ASCOMResult};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) struct Sender {
    pub(crate) client: Arc<Client>,
    pub(crate) unique_id: String,
    pub(crate) device_type: DeviceType,
    pub(crate) device_number: usize,
}

impl Sender {
    pub(crate) async fn exec_action<Resp>(
        &self,
        is_mut: bool,
        action: &str,
        params: OpaqueParams<str>,
    ) -> ASCOMResult<Resp>
    where
        ASCOMResult<Resp>: Response,
    {
        self.client
            .request::<ASCOMResult<Resp>>(
                &format!(
                    "api/v1/{device_type}/{device_number}/{action}",
                    device_type = self.device_type,
                    device_number = self.device_number
                ),
                is_mut,
                params,
                |request| request,
            )
            .await
            .unwrap_or_else(|err| {
                Err(ASCOMError::new(
                    ASCOMErrorCode::UNSPECIFIED,
                    format!("{err:#}"),
                ))
            })
    }
}
