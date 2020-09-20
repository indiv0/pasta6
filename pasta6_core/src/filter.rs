use crate::{
    Error::{self, DbPoolError},
    User,
};
use deadpool_postgres::{Client, Pool};
use warp::{any, reject, Filter, Rejection};
pub struct TemplateContext<U>
where
    U: User,
{
    current_user: Option<U>,
    domain: String,
}

impl<U> TemplateContext<U>
where
    U: User,
{
    pub fn new(current_user: Option<U>, domain: String) -> Self {
        Self {
            current_user,
            domain,
        }
    }

    pub fn current_user(&self) -> Option<&U> {
        self.current_user.as_ref()
    }

    pub fn domain(&self) -> &str {
        &self.domain
    }
}

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

pub async fn get_db_connection(pool: &Pool) -> Result<Client, Error> {
    pool.get().await.map_err(DbPoolError)
}
