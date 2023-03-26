use super::ActionParams;
use crate::macros::auto_increment;
use serde::Serialize;
use std::num::NonZeroU32;

#[derive(Debug, Serialize, Clone, Copy)]
pub(crate) struct ResponseTransaction {
    #[serde(rename = "ClientTransactionID")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) client_transaction_id: Option<NonZeroU32>,

    #[serde(rename = "ServerTransactionID")]
    pub(crate) server_transaction_id: NonZeroU32,
}

impl ResponseTransaction {
    pub(crate) fn new(client_transaction_id: Option<NonZeroU32>) -> Self {
        Self {
            client_transaction_id,
            server_transaction_id: auto_increment!(),
        }
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct ResponseWithTransaction<T> {
    #[serde(flatten)]
    pub(crate) transaction: ResponseTransaction,
    #[serde(flatten)]
    pub(crate) response: T,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct RequestTransaction {
    pub(crate) client_id: Option<NonZeroU32>,
    pub(crate) client_transaction_id: Option<NonZeroU32>,
}

impl RequestTransaction {
    pub(crate) fn extract(params: &mut ActionParams) -> anyhow::Result<Self> {
        let mut extract_id = |name| {
            match params {
                ActionParams::Get(params) => params.maybe_extract(name),
                ActionParams::Put(params) => params.maybe_extract(name),
            }
            .map(|maybe_id| maybe_id.and_then(NonZeroU32::new))
        };

        Ok(Self {
            client_id: extract_id("ClientID")?,
            client_transaction_id: extract_id("ClientTransactionID")?,
        })
    }
}
