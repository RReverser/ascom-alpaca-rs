
use crate::params::OpaqueParams;
use crate::response::OpaqueResponse;
use crate::transaction::Client;
use crate::{ASCOMError, ASCOMErrorCode, ASCOMResult};


use std::sync::Arc;


#[derive(Debug)]
pub(crate) struct Sender {
    pub(crate) client: Arc<Client>,
    pub(crate) unique_id: String,
    pub(crate) device_number: usize,
}

impl Sender {
    pub(crate) async fn exec_action(
        &self,
        device_type: &str,
        is_mut: bool,
        action: &str,
        params: OpaqueParams,
    ) -> ASCOMResult<OpaqueResponse> {
        self.client
            .request::<ASCOMResult<OpaqueResponse>>(
                &format!(
                    "api/v1/{device_type}/{device_number}/{action}",
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
