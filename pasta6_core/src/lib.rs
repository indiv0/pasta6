#[cfg(test)]
#[macro_use]
extern crate assert_matches;
#[macro_use]
extern crate lazy_static;

use deadpool_postgres::{
    config::ConfigError, Config as PoolConfig, ManagerConfig, Pool, RecyclingMethod,
};
use tokio_postgres::NoTls;

pub use auth::{
    optional_session, optional_user, row_to_user, BaseUser, CoreUserStore, Session, User,
    UserStore, SESSION_COOKIE_NAME,
};
pub use config::Config;
pub use error::{Error, ErrorResponse};
pub use filter::{get_db_connection, with_db, TemplateContext};
pub use routes::form_body;
pub use server::{bind, init_server, init_server2, init_tracing};
use tracing::trace;

mod auth;
mod config;
mod error;
mod filter;
mod routes;
mod server;

lazy_static! {
    pub static ref CONFIG: Config = Config::load();
}
pub trait Context {
    fn config(&self) -> &'static Config {
        &*CONFIG
    }
}

pub fn create_db_pool(site: &str) -> Result<Pool, ConfigError> {
    let mut cfg = PoolConfig::new();
    cfg.host = Some(
        CONFIG
            .get(&format!("services.{}.database.host", site))
            .unwrap()
            .to_owned(),
    );
    cfg.user = Some(
        CONFIG
            .get(&format!("services.{}.database.user", site))
            .unwrap()
            .to_owned(),
    );
    cfg.password = Some(
        CONFIG
            .get(&format!("services.{}.database.password", site))
            .unwrap()
            .to_owned(),
    );
    cfg.dbname = Some(
        CONFIG
            .get(&format!("services.{}.database.dbname", site))
            .unwrap()
            .to_owned(),
    );
    trace!("Creating database pool: host={}, user={}, dbname={}", cfg.host.as_ref().unwrap(), cfg.user.as_ref().unwrap(), cfg.dbname.as_ref().unwrap());
    cfg.manager = Some(ManagerConfig {
        recycling_method: RecyclingMethod::Fast,
    });
    cfg.create_pool(NoTls)
}
