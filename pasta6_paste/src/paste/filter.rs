use crate::paste::db;
use crate::paste::models::{Paste, PasteForm};
use askama_warp::Template;
use deadpool_postgres::Client;
use db::Hash;
use pasta6_core::Error::DbQueryError;
use pasta6_core::{Context, CoreUser, TemplateContext, User};
use std::str::FromStr;
use warp::redirect::redirect;
use warp::{http::Uri, reject::custom, Rejection, Reply};

pub(crate) async fn create_paste(body: PasteForm, client: Client) -> Result<impl Reply, Rejection> {
    let paste = db::create_paste(&client, body.data())
        .await
        .map_err(DbQueryError)
        .map_err(|e| custom(e))?;
    assert_eq!(paste.data().as_bytes(), body.data());
    let redirect_uri = Uri::from_str(&format!("/paste/{hash}", hash = paste.hash())).unwrap();
    // TODO: 302 instead of 301 here
    Ok(redirect(redirect_uri))
}

#[derive(Template)]
#[template(path = "paste.html")]
struct PasteTemplate {
    ctx: TemplateContext<CoreUser>,
    paste: Paste,
}

pub(crate) async fn get_paste(
    hash: Hash,
    client: Client,
    // TODO: we don't actually need the username for this endpoint until
    // _after_ we've done `db::get_paste` (that is, the ctx is necessary for
    // the response only). So perhaps we could optimize away the DB query by
    // only doing it afterwards?
    current_user: Option<CoreUser>,
) -> Result<impl Reply, Rejection> {
    let paste = db::get_paste(&client, hash)
        .await
        .map_err(DbQueryError)
        .map_err(|e| custom(e))?;
    Ok(PasteTemplate {
        ctx: TemplateContext::new(current_user),
        paste,
    })
}
