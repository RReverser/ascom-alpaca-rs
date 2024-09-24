use crate::macros::auto_increment;
use serde::{Deserialize, Serialize};
use std::num::NonZeroU32;

#[derive(Debug, Serialize, Clone, Copy)]
pub(crate) struct RequestTransaction {
    #[serde(rename = "ClientTransactionID")]
    pub(crate) client_transaction_id: NonZeroU32,
    #[serde(rename = "ClientID")]
    pub(crate) client_id: NonZeroU32,
}

impl RequestTransaction {
    pub(crate) fn new(client_id: NonZeroU32) -> Self {
        Self {
            client_transaction_id: auto_increment!(),
            client_id,
        }
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct RequestWithTransaction<T> {
    #[serde(flatten)]
    pub(crate) transaction: RequestTransaction,
    #[serde(flatten)]
    pub(crate) params: T,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub(crate) struct ResponseTransaction {
    #[serde(rename = "ClientTransactionID")]
    pub(crate) client_transaction_id: Option<NonZeroU32>,
    #[serde(rename = "ClientID")]
    pub(crate) server_transaction_id: Option<NonZeroU32>,
}

#[derive(Debug)]
pub(crate) struct ResponseWithTransaction<T> {
    pub(crate) transaction: ResponseTransaction,
    pub(crate) response: T,
}
