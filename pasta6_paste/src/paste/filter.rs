use crate::paste::db;
use crate::paste::models::{self, Paste, PasteCreateResponse, PasteForm};
use askama_warp::Template;
use deadpool_postgres::Client as DbClient;
use pasta6_core::{TemplateContext, User, BaseUser};
use std::str::FromStr;
use warp::http::Uri;

pub(crate) async fn create_paste_api(
    body: bytes::Bytes,
    db: DbClient,
) -> Result<impl warp::Reply, warp::Rejection> {
    Ok(warp::reply::json(&PasteCreateResponse::of(
        db::create_paste(&db, &body[..])
            .await
            .map_err(|e| warp::reject::custom(e))?,
    )))
}

pub(crate) async fn get_paste_api(id: i32, db: DbClient) -> Result<impl warp::Reply, warp::Rejection> {
    Ok(models::paste_to_paste_get_response(
        db::get_paste(&db, id)
            .await
            .map_err(|e| warp::reject::custom(e))?,
    ))
}

pub(crate) async fn create_paste(
    body: PasteForm,
    db: DbClient,
) -> Result<impl warp::Reply, warp::Rejection> {
    let paste = db::create_paste(&db, body.data())
        .await
        .map_err(|e| warp::reject::custom(e))?;
    assert_eq!(paste.data().as_bytes(), body.data());
    let redirect_uri = Uri::from_str(&format!("/paste/{id}", id = paste.id())).unwrap();
    // TODO: 302 instead of 301 here
    Ok(warp::redirect::redirect(redirect_uri))
}

#[derive(Template)]
#[template(path = "paste.html")]
struct PasteTemplate {
    ctx: TemplateContext<BaseUser>,
    _paste: Paste,
}

pub(crate) async fn get_paste(
    id: i32,
    db: DbClient,
    // TODO: we don't actually need the username for this endpoint until
    // _after_ we've done `db::get_paste` (that is, the ctx is necessary for
    // the response only). So perhaps we could optimize away the DB query by
    // only doing it afterwards?
    current_user: Option<BaseUser>,
) -> Result<impl warp::Reply, warp::Rejection> {
    let paste = db::get_paste(&db, id)
        .await
        .map_err(|e| warp::reject::custom(e))?;
    Ok(PasteTemplate {
        ctx: TemplateContext::new(current_user),
        _paste: paste,
    })
}
