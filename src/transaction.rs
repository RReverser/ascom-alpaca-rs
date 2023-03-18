use crate::params::OpaqueParams;
use crate::response::OpaqueResponse;
use serde::Serialize;
use std::sync::atomic::{AtomicU32, Ordering};

macro_rules! auto_increment {
    () => {{
        static COUNTER: AtomicU32 = AtomicU32::new(1);
        COUNTER.fetch_add(1, Ordering::Relaxed)
    }};
}

#[derive(Debug, Serialize, Clone, Copy)]
pub(crate) struct ServerResponseTransaction {
    #[serde(rename = "ClientTransactionID")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) client_transaction_id: Option<u32>,

    #[serde(rename = "ServerTransactionID")]
    pub(crate) server_transaction_id: u32,
}

impl ServerResponseTransaction {
    pub(crate) fn new(client_transaction_id: Option<u32>) -> Self {
        Self {
            client_transaction_id,
            server_transaction_id: auto_increment!(),
        }
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct ServerResponseWithTransaction {
    #[serde(flatten)]
    pub(crate) transaction: ServerResponseTransaction,
    #[serde(flatten)]
    pub(crate) response: OpaqueResponse,
}

#[derive(Debug, Serialize, Clone, Copy)]
pub(crate) struct ClientRequestTransaction {
    pub(crate) client_transaction_id: u32,
    pub(crate) client_id: u32,
}

impl ClientRequestTransaction {
    pub(crate) fn new(client_id: u32) -> Self {
        Self {
            client_transaction_id: auto_increment!(),
            client_id,
        }
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct ClientRequestWithTransaction {
    #[serde(flatten)]
    pub(crate) transaction: ClientRequestTransaction,
    #[serde(flatten)]
    pub(crate) params: OpaqueParams<str>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ClientResponseTransaction {
    pub(crate) client_transaction_id: Option<u32>,
    pub(crate) server_transaction_id: Option<u32>,
}
