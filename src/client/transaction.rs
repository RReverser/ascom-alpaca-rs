use super::ActionParams;
use crate::macros::auto_increment;
use crate::response::OpaqueResponse;
use anyhow::Context;
use serde::Serialize;

#[derive(Debug, Serialize, Clone, Copy)]
pub(crate) struct RequestTransaction {
    pub(crate) client_transaction_id: u32,
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
pub(crate) struct RequestWithTransaction {
    #[serde(flatten)]
    pub(crate) transaction: RequestTransaction,
    #[serde(flatten)]
    pub(crate) params: ActionParams,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ResponseTransaction {
    pub(crate) client_transaction_id: Option<u32>,
    pub(crate) server_transaction_id: Option<u32>,
}

impl ResponseTransaction {
    pub(crate) fn extract(response: &mut OpaqueResponse) -> anyhow::Result<Self> {
        let mut extract_id = |name| {
            response
                .0
                .remove(name)
                .map(serde_json::from_value)
                .transpose()
                .with_context(|| format!("couldn't parse {name}"))
        };

        Ok(Self {
            client_transaction_id: extract_id("ClientTransactionID")?,
            server_transaction_id: extract_id("ServerTransactionID")?,
        })
    }
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

impl TryFrom<OpaqueResponse> for ResponseWithTransaction<OpaqueResponse> {
    type Error = anyhow::Error;

    fn try_from(mut response: OpaqueResponse) -> anyhow::Result<Self> {
        let transaction = ResponseTransaction::extract(&mut response)?;

        Ok(Self {
            transaction,
            response,
        })
    }
}
