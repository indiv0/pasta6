use crate::paste::db;
use crate::paste::models::{self, Paste, PasteCreateResponse, PasteForm};
use askama_warp::Template;
use bytes::Bytes;
use deadpool_postgres::Client;
use models::paste_to_paste_get_response;
use pasta6_core::Error::DbQueryError;
use pasta6_core::{Context, CoreUser, TemplateContext, User};
use std::str::FromStr;
use warp::redirect::redirect;
use warp::{http::Uri, reject::custom, reply::json, Rejection, Reply};

pub(crate) async fn create_paste_api(body: Bytes, client: Client) -> Result<impl Reply, Rejection> {
    Ok(json(&PasteCreateResponse::of(
        db::create_paste(&client, &body[..])
            .await
            .map_err(DbQueryError)
            .map_err(|e| custom(e))?,
    )))
}

pub(crate) async fn get_paste_api(id: i32, client: Client) -> Result<impl Reply, Rejection> {
    Ok(paste_to_paste_get_response(
        db::get_paste(&client, id)
            .await
            .map_err(DbQueryError)
            .map_err(|e| custom(e))?,
    ))
}

pub(crate) async fn create_paste(body: PasteForm, client: Client) -> Result<impl Reply, Rejection> {
    let paste = db::create_paste(&client, body.data())
        .await
        .map_err(DbQueryError)
        .map_err(|e| custom(e))?;
    assert_eq!(paste.data().as_bytes(), body.data());
    let redirect_uri = Uri::from_str(&format!("/paste/{id}", id = paste.id())).unwrap();
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
    id: i32,
    client: Client,
    // TODO: we don't actually need the username for this endpoint until
    // _after_ we've done `db::get_paste` (that is, the ctx is necessary for
    // the response only). So perhaps we could optimize away the DB query by
    // only doing it afterwards?
    current_user: Option<CoreUser>,
) -> Result<impl Reply, Rejection> {
    let paste = db::get_paste(&client, id)
        .await
        .map_err(DbQueryError)
        .map_err(|e| custom(e))?;
    Ok(PasteTemplate {
        ctx: TemplateContext::new(current_user),
        paste,
    })
}
