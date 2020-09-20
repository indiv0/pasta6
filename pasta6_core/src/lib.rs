use deadpool_postgres::{config::ConfigError, Config, ManagerConfig, Pool, RecyclingMethod};
use std::env;
use tokio_postgres::NoTls;

pub use auth::{
    optional_session, optional_user, row_to_user, BaseUser, CoreUserStore, Session, User,
    UserStore, SESSION_COOKIE_NAME,
};
pub use error::{Error, ErrorResponse};
pub use filter::{get_db_connection, with_db, TemplateContext};
pub use routes::form_body;
pub use server::{bind, init_server, init_server2, init_tracing};

mod auth;
mod error;
mod filter;
mod routes;
mod server;

pub fn create_db_pool() -> Result<Pool, ConfigError> {
    let mut cfg = Config::new();
    cfg.host = Some(env::var("POSTGRES_HOST").expect("POSTGRES_HOST unset"));
    cfg.user = Some(env::var("POSTGRES_USER").expect("POSTGRES_USER unset"));
    cfg.password = Some(env::var("POSTGRES_PASSWORD").expect("POSTGRES_PASSWORD unset"));
    cfg.dbname = Some(env::var("POSTGRES_DB").expect("POSTGRES_DB unset"));
    cfg.manager = Some(ManagerConfig {
        recycling_method: RecyclingMethod::Fast,
    });
    cfg.create_pool(NoTls)
}
