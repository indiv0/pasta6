use std::convert::Infallible;

use crate::{
    Context,
    Error::{self, DbPoolError},
    SecretKey, Token, User,
};
use deadpool_postgres::{Client, Pool};
use serde::de::DeserializeOwned;
use tracing::error;
use warp::{any, body::content_length_limit, body::form, cookie, reject, Filter, Rejection};

pub const SESSION_COOKIE_NAME: &str = "__Secure-p6.rs.login.v1";
const MAX_CONTENT_LENGTH: u64 = 1026 * 16; // 16KB

pub struct TemplateContext<U>
where
    U: User,
{
    current_user: Option<U>,
}

impl<U> TemplateContext<U>
where
    U: User,
{
    pub fn new(current_user: Option<U>) -> Self {
        Self { current_user }
    }

    pub fn current_user(&self) -> Option<&U> {
        self.current_user.as_ref()
    }
}

impl<U> Context for TemplateContext<U> where U: User {}

pub fn with_db(pool: Pool) -> impl Filter<Extract = (Client,), Error = Rejection> + Clone {
    any().and_then(move || {
        let pool = pool.clone();
        async move {
            get_db_connection(&pool)
                .await
                .map_err(|e| reject::custom(e))
        }
    })
}

pub fn form_body<T>() -> impl Filter<Extract = (T,), Error = Rejection> + Clone
where
    T: Send + DeserializeOwned,
{
    content_length_limit(MAX_CONTENT_LENGTH).and(form())
}

pub async fn get_db_connection(pool: &Pool) -> Result<Client, Error> {
    pool.get().await.map_err(DbPoolError)
}

pub fn with_token(
    secret_key: SecretKey,
    ttl: u32,
) -> impl Filter<Extract = (Option<Token>,), Error = Infallible> + Clone {
    any().and(cookie::optional(SESSION_COOKIE_NAME)).and_then(
        move |maybe_cookie: Option<String>| {
            let secret_key = secret_key.clone();
            async move {
                Ok(match maybe_cookie {
                    Some(cookie) => {
                        // TODO: we should be unsetting the cookie entirely rather than leaving it as a blank string.
                        if cookie.is_empty() {
                            None
                        } else {
                            let token = match bronco::decode(&cookie, secret_key.as_bytes(), ttl) {
                                Ok(token) => token,
                                Err(e) => {
                                    error!("token decoding failed: {:?}", e);
                                    return Ok::<_, Infallible>(None);
                                }
                            };
                            // FIXME: remove this unwrap
                            let token: Token = serde_json::from_str(&token).unwrap();
                            Some(token)
                        }
                    }
                    None => None,
                })
            }
        },
    )
}
