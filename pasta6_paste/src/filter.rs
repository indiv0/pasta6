use askama_warp::Template;
use deadpool_postgres::Client;
use pasta6_core::Error::DbQueryError;
use pasta6_core::{Context, CoreUser, Error, ErrorResponse, TemplateContext, User};
use std::convert::Infallible;
use tracing::error;
use warp::{
    body::BodyDeserializeError, http::StatusCode, reject::custom, reject::MethodNotAllowed,
    reply::json, reply::with_status, Rejection, Reply,
};

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    ctx: TemplateContext<CoreUser>,
}

// TODO: only get a DB connection if the session is present.
pub(crate) async fn index(current_user: Option<CoreUser>) -> Result<impl Reply, Rejection> {
    Ok(IndexTemplate {
        ctx: TemplateContext::new(current_user),
    })
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
            DbQueryError(e) => {
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
