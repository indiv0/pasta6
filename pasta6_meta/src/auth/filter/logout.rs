use deadpool_postgres::Client as DbClient;
use pasta6_core::Session;
use warp::{redirect, http::Uri, reply::with_header};
use warp::http::header;
use crate::auth::db;

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
        .map(|reply| with_header(reply, header::SET_COOKIE, ""))
}