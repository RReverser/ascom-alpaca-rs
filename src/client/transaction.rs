use super::ActionParams;
use crate::macros::auto_increment;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Clone, Copy)]
pub(crate) struct RequestTransaction {
    #[serde(rename = "ClientTransactionID")]
    pub(crate) client_transaction_id: u32,
    #[serde(rename = "ClientID")]
    pub(crate) client_id: u32,
}

impl RequestTransaction {
    pub(crate) fn new(client_id: u32) -> Self {
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
    pub(crate) params: ActionParams<T>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub(crate) struct ResponseTransaction {
    #[serde(rename = "ClientTransactionID")]
    pub(crate) client_transaction_id: Option<u32>,
    #[serde(rename = "ClientID")]
    pub(crate) server_transaction_id: Option<u32>,
}

#[derive(Debug)]
pub(crate) struct ResponseWithTransaction<T> {
    pub(crate) transaction: ResponseTransaction,
    pub(crate) response: T,
}

impl<T> ResponseWithTransaction<T> {
    pub(crate) fn map<T2>(self, f: impl FnOnce(T) -> T2) -> ResponseWithTransaction<T2> {
        ResponseWithTransaction {
            transaction: self.transaction,
            response: f(self.response),
        }
    }
}
