use self::Error::{DbPoolError, DbQueryError, SerdeJsonError};
use serde::Serialize;
use std::fmt::{Display, Formatter, Result};
use warp::reject::Reject;

#[derive(Debug)]
pub enum Error {
    // TODO: remove the `Error` suffix from all the variants.
    SerdeJsonError(serde_json::error::Error),
    DbPoolError(deadpool_postgres::PoolError),
    DbQueryError(tokio_postgres::Error),
}

impl From<serde_json::error::Error> for Error {
    fn from(err: serde_json::error::Error) -> Self {
        SerdeJsonError(err)
    }
}

impl From<deadpool_postgres::PoolError> for Error {
    fn from(err: deadpool_postgres::PoolError) -> Self {
        DbPoolError(err)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match *self {
            SerdeJsonError(ref e) => write!(f, "error serializing/deserializing JSON data: {0}", e),
            DbPoolError(ref e) => write!(f, "error getting connection from DB pool: {0}", e),
            DbQueryError(ref e) => write!(f, "error executing DB query: {0}", e),
        }
    }
}

impl Reject for Error {}
impl std::error::Error for Error {}

#[derive(Serialize)]
pub struct ErrorResponse {
    message: String,
}

impl ErrorResponse {
    pub fn new(message: String) -> Self {
        Self { message }
    }
}
