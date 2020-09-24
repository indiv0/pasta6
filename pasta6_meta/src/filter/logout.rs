use super::set_session;
use deadpool_postgres::Client;
use warp::{http::header, Rejection, Reply};
use warp::{http::Uri, redirect, reply::with_header};

// TODO: only allow this endpoint if there user is set
// TODO: we don't need the DB client to logout.
pub(crate) async fn get_logout(_client: Client) -> Result<impl Reply, Rejection> {
    Ok(redirect(Uri::from_static("/")))
        // TODO: should we specify `Domain={...}; HttpOnly;`, etc. when unsetting the cookie?
        // TODO: should we be nuking the whole cookie?
        .map(|reply| with_header(reply, header::SET_COOKIE, set_session("")))
}
