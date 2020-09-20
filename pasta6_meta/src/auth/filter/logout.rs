use super::set_session;
use crate::auth::db;
use deadpool_postgres::Client as DbClient;
use pasta6_core::Session;
use warp::http::header;
use warp::{http::Uri, redirect, reply::with_header};

pub(crate) async fn get_logout(
    session: Option<Session>,
    db: DbClient,
) -> Result<impl warp::Reply, warp::Rejection> {
    if let Some(s) = session {
        db::unset_session(&db, &s)
            .await
            .map_err(|e| warp::reject::custom(e))?;
    }

    Ok(redirect(Uri::from_static("/")))
        // TODO: should we specify `Domain={...}; HttpOnly;`, etc. when unsetting the cookie?
        // TODO: should we be nuking the whole cookie?
        .map(|reply| with_header(reply, header::SET_COOKIE, set_session("")))
}
