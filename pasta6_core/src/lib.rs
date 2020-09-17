use deadpool_postgres::{Config, ManagerConfig, RecyclingMethod, Pool, config::ConfigError};
use std::env;
use tokio_postgres::NoTls;

pub use auth::{UserStore, CoreUserStore, optional_user, optional_session, row_to_user, Session, SESSION_COOKIE_NAME, BaseUser, User};
pub use error::{Error, ErrorResponse};
pub use filter::{get_db_connection, with_db, TemplateContext};
pub use routes::form_body;
pub use server::{init_server, init_server2, bind, init_tracing};

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