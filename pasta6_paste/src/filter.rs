use crate::DOMAIN;
use askama_warp::Template;
use deadpool_postgres::Client as DbClient;
use pasta6_core::{BaseUser, Error, ErrorResponse, TemplateContext, User};
use std::convert::Infallible;
use warp::http::StatusCode;

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    ctx: TemplateContext<BaseUser>,
}

// TODO: only get a DB connection if the session is present.
pub(crate) async fn index(
    current_user: Option<BaseUser>,
) -> Result<impl warp::Reply, warp::Rejection> {
    Ok(IndexTemplate {
        ctx: TemplateContext::new(current_user, DOMAIN.to_owned()),
    })
}

pub(crate) async fn health(db: DbClient) -> Result<impl warp::Reply, warp::Rejection> {
    // Check if our connection to the DB is still OK.
    db.query("SELECT 1", &[])
        .await
        .map_err(|e| warp::reject::custom(Error::DbQueryError(e)))?;

    Ok(StatusCode::OK)
}

pub(crate) async fn handle_rejection(err: warp::Rejection) -> Result<impl warp::Reply, Infallible> {
    let code;
    let message;

    if err.is_not_found() {
        code = StatusCode::NOT_FOUND;
        message = "Not found";
    } else if let Some(e) = err.find::<warp::filters::body::BodyDeserializeError>() {
        // TODO: disable this log line outside of development
        eprintln!("body deserialize error: {:?}", e);
        code = StatusCode::BAD_REQUEST;
        message = "Invalid body";
    } else if let Some(e) = err.find::<Error>() {
        match e {
            Error::DbQueryError(e) => {
                eprintln!("could not execute request: {:?}", e);
                code = StatusCode::BAD_REQUEST;
                message = "Could not execute request";
            }
            _ => {
                eprintln!("unhandled application error: {:?}", err);
                code = warp::http::StatusCode::INTERNAL_SERVER_ERROR;
                message = "Internal server error";
            }
        }
    } else if let Some(_) = err.find::<warp::reject::MethodNotAllowed>() {
        code = StatusCode::METHOD_NOT_ALLOWED;
        message = "Method not allowed";
    } else {
        eprintln!("unhandled error: {:?}", err);
        code = StatusCode::INTERNAL_SERVER_ERROR;
        message = "Internal server error";
    }

    let json = warp::reply::json(&ErrorResponse::new(message.into()));

    Ok(warp::reply::with_status(json, code))
}
