use crate::CONFIG;
use pasta6_core::{Config, Session, SESSION_COOKIE_NAME};
use rand::{distributions::Alphanumeric, thread_rng, Rng};

mod login;
mod logout;
mod profile;
mod register;

pub(crate) use login::{get_login, post_login};
pub(crate) use logout::get_logout;
pub(crate) use profile::get_profile;
pub(crate) use register::{get_register, post_register};

const SESSION_ID_LENGTH: usize = 30;

fn generate_random_session() -> Session {
    // TODO: generate the session ID in a cryptographically secure way.
    let id = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(SESSION_ID_LENGTH)
        .collect();
    Session::new(id)
}

fn set_session(value: &str) -> String {
    assert!(SESSION_COOKIE_NAME.starts_with("__Secure-"));
    format!(
        "{}={}; Domain={}; Secure; HttpOnly; SameSite=Strict",
        SESSION_COOKIE_NAME,
        value,
        CONFIG.domain()
    )
}
