use crate::params::ActionParams;
use crate::response::OpaqueResponse;
use serde::Serialize;

#[derive(Debug, Serialize, Clone, Copy)]
pub(crate) struct ResponseTransaction {
    #[serde(rename = "ClientTransactionID")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) client_transaction_id: Option<u32>,

    #[serde(rename = "ServerTransactionID")]
    pub(crate) server_transaction_id: u32,
}

impl ResponseTransaction {
    pub(crate) fn new(client_transaction_id: Option<u32>) -> Self {
        Self {
            client_transaction_id,
            server_transaction_id: auto_increment!(),
        }
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct ResponseWithTransaction {
    #[serde(flatten)]
    pub(crate) transaction: ResponseTransaction,
    #[serde(flatten)]
    pub(crate) response: OpaqueResponse,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct RequestTransaction {
    pub(crate) client_id: Option<u32>,
    pub(crate) client_transaction_id: Option<u32>,
}

impl RequestTransaction {
    pub(crate) fn extract(params: &mut ActionParams) -> anyhow::Result<Self> {
        let mut extract_id = |name| match params {
            ActionParams::Get(params) => params.maybe_extract(name),
            ActionParams::Put(params) => params.maybe_extract(name),
        };

        Ok(Self {
            client_id: extract_id("ClientID")?,
            client_transaction_id: extract_id("ClientTransactionID")?,
        })
    }
}
