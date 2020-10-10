use crate::config::ConfigError;

use serde::Serialize;
use std::fmt::{Display, Formatter, Result};
use warp::reject::Reject;

#[derive(Debug)]
pub enum Error {
    // TODO: remove the `Error` suffix from all the variants.
    SerdeJsonError(serde_json::error::Error),
    DbPoolError(deadpool_postgres::PoolError),
    DbQueryError(tokio_postgres::Error),
    ConfigError(ConfigError),
}

impl From<serde_json::error::Error> for Error {
    fn from(err: serde_json::error::Error) -> Self {
        Self::SerdeJsonError(err)
    }
}

impl From<deadpool_postgres::PoolError> for Error {
    fn from(err: deadpool_postgres::PoolError) -> Self {
        Self::DbPoolError(err)
    }
}

impl From<ConfigError> for Error {
    fn from(err: ConfigError) -> Self {
        Self::ConfigError(err)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match *self {
            Self::SerdeJsonError(ref e) => {
                write!(f, "error serializing/deserializing JSON data: {0}", e)
            }
            Self::DbPoolError(ref e) => write!(f, "error getting connection from DB pool: {0}", e),
            Self::DbQueryError(ref e) => write!(f, "error executing DB query: {0}", e),
            Self::ConfigError(ref e) => write!(f, "error reading value from config: {0}", e),
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
