use crate::{User, Error::{self, DbPoolError}};
use warp::{any, reject, Rejection, Filter};
use deadpool_postgres::{Client, Pool};
pub struct TemplateContext {
    current_user: Option<User>,
}

impl TemplateContext {
    pub fn new(current_user: Option<User>) -> Self {
        Self { current_user }
    }

    pub fn current_user(&self) -> Option<&User> {
        self.current_user.as_ref()
    }
}

pub fn with_db(
    pool: Pool,
) -> impl Filter<Extract = (Client,), Error = Rejection> + Clone {
    any()
        .and_then(move || {
            let pool = pool.clone();
            async move {
                get_db_connection(&pool)
                    .await
                    .map_err(|e| reject::custom(e))
            }
        })
}

pub async fn get_db_connection(pool: &Pool) -> Result<Client, Error> {
    pool.get().await.map_err(DbPoolError)
}