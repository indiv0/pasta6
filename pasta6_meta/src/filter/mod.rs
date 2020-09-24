mod login;
mod logout;
mod profile;
mod register;

use std::convert::Infallible;

use askama_warp::Template;
use deadpool_postgres::Client;
pub(crate) use login::{get_login, post_login};
pub(crate) use logout::get_logout;
use pasta6_core::Error::DbQueryError;
use pasta6_core::{
    Context, Error, ErrorResponse, TemplateContext, User, CONFIG, SESSION_COOKIE_NAME,
};
pub(crate) use profile::get_profile;
pub(crate) use register::{get_register, post_register};
use tracing::error;
use warp::{
    body::BodyDeserializeError, http::StatusCode, reject::MethodNotAllowed, reply::json,
    reply::with_status,
};
use warp::{reject::custom, Rejection, Reply};

fn set_session(value: &str) -> String {
    assert!(SESSION_COOKIE_NAME.starts_with("__Secure-"));
    format!(
        "{}={}; Domain={}; Secure; HttpOnly; SameSite=Strict",
        SESSION_COOKIE_NAME,
        value,
        CONFIG.get("pasta6.domain").unwrap()
    )
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate<U>
where
    U: User + Send,
{
    ctx: TemplateContext<U>,
}

// TODO: only get a DB connection if the session is present.
pub(crate) async fn index<U>(ctx: TemplateContext<U>) -> Result<impl Reply, Rejection>
where
    U: User + Send,
{
    Ok(IndexTemplate { ctx })
}

pub(crate) async fn health(client: Client) -> Result<impl Reply, Rejection> {
    // Check if our connection to the DB is still OK.
    client
        .query("SELECT 1", &[])
        .await
        .map_err(DbQueryError)
        .map_err(|e| custom(e))?;

    Ok(StatusCode::OK)
}

pub(crate) async fn handle_rejection(err: Rejection) -> Result<impl Reply, Infallible> {
    let code;
    let message;

    if err.is_not_found() {
        code = StatusCode::NOT_FOUND;
        message = "Not found";
    } else if let Some(e) = err.find::<BodyDeserializeError>() {
        // TODO: disable this log line outside of development
        error!("body deserialize error: {:?}", e);
        code = StatusCode::BAD_REQUEST;
        message = "Invalid body";
    } else if let Some(e) = err.find::<Error>() {
        match e {
            Error::DbQueryError(e) => {
                error!("could not execute request: {:?}", e);
                code = StatusCode::BAD_REQUEST;
                message = "Could not execute request";
            }
            _ => {
                error!("unhandled application error: {:?}", err);
                code = StatusCode::INTERNAL_SERVER_ERROR;
                message = "Internal server error";
            }
        }
    } else if let Some(_) = err.find::<MethodNotAllowed>() {
        code = StatusCode::METHOD_NOT_ALLOWED;
        message = "Method not allowed";
    } else {
        error!("unhandled error: {:?}", err);
        code = StatusCode::INTERNAL_SERVER_ERROR;
        message = "Internal server error";
    }

    let json = json(&ErrorResponse::new(message.into()));

    Ok(with_status(json, code))
}
