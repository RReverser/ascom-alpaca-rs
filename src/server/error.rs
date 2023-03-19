use crate::ASCOMError;
use thiserror::Error;

#[derive(Error, Debug)]
pub(crate) enum Error {
    #[error("Bad request: {0}")]
    BadRequest(#[source] anyhow::Error),
    #[error("Bad request: {0}")]
    NotFound(#[source] anyhow::Error),
    #[error(transparent)]
    Ascom(#[from] ASCOMError),
}
