use chrono::{DateTime, Utc};
use serde_derive::Deserialize;

use super::db::Hash;

#[derive(Deserialize)]
#[serde(transparent)]
pub(crate) struct Form<T>(T);

#[derive(Deserialize)]
pub(crate) struct PasteForm {
    data: String,
}

impl PasteForm {
    pub(crate) fn data(&self) -> &[u8] {
        self.data.as_bytes()
    }
}

#[derive(Debug)]
pub(crate) struct Paste {
    id: i32,
    created_at: DateTime<Utc>,
    hash: Hash,
    data: Vec<u8>,
    user_id: i32,
}

impl Paste {
    pub(crate) fn new(id: i32, created_at: DateTime<Utc>, hash: Hash, data: Vec<u8>, user_id: i32) -> Self {
        Self {
            id,
            created_at,
            hash,
            data,
            user_id,
        }
    }

    pub(crate) fn _id(&self) -> &i32 {
        &self.id
    }

    pub(crate) fn created_at(&self) -> &DateTime<Utc> {
        &self.created_at
    }

    pub(crate) fn hash(&self) -> &Hash {
        &self.hash
    }

    pub(crate) fn data(&self) -> &str {
        &std::str::from_utf8(&self.data).unwrap()
    }
}