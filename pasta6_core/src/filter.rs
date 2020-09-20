use crate::{
    Error::{self, DbPoolError},
    User,
};
use deadpool_postgres::{Client, Pool};
use serde::Deserialize;
use warp::{any, reject, Filter, Rejection};

pub trait Config {
    fn domain(&self) -> &str;
}

#[derive(Deserialize)]
pub struct CoreConfig {
    domain: String,
}

impl Config for CoreConfig {
    fn domain(&self) -> &str {
        &self.domain
    }
}

pub struct TemplateContext<'a, C, U>
where
    C: Config,
    U: User,
{
    config: &'a C,
    current_user: Option<U>,
}

impl<'a, C, U> TemplateContext<'a, C, U>
where
    C: Config,
    U: User,
{
    pub fn new(config: &'a C, current_user: Option<U>) -> Self {
        Self {
            config,
            current_user,
        }
    }

    pub fn config(&self) -> &impl Config {
        self.config
    }

    pub fn current_user(&self) -> Option<&U> {
        self.current_user.as_ref()
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
