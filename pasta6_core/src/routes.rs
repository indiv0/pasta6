use serde::de::DeserializeOwned;
use warp::{
    body::{content_length_limit, form},
    Filter, Rejection,
};

const MAX_CONTENT_LENGTH: u64 = 1026 * 16; // 16KB

pub fn form_body<T>() -> impl Filter<Extract = (T,), Error = Rejection> + Clone
where
    T: Send + DeserializeOwned,
{
    content_length_limit(MAX_CONTENT_LENGTH).and(form())
}
