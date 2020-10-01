use chrono::{DateTime, Utc};
use serde_derive::{Deserialize, Serialize};

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
}

impl Paste {
    pub(crate) fn new(id: i32, created_at: DateTime<Utc>, hash: Hash, data: Vec<u8>) -> Self {
        Self {
            id,
            created_at,
            hash,
            data,
        }
    }

    pub(crate) fn id(&self) -> &i32 {
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

#[derive(Serialize)]
pub(crate) struct PasteCreateResponse {
    id: i32,
}

impl PasteCreateResponse {
    // TODO: should this be implemented with `Into` or `From`?
    pub(crate) fn of(paste: Paste) -> Self {
        Self { id: paste.id }
    }
}

type PasteGetResponse = Vec<u8>;

pub(crate) fn paste_to_paste_get_response(paste: Paste) -> PasteGetResponse {
    paste.data
}
