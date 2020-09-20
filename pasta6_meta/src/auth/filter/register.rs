use super::{generate_random_session, set_session};
use crate::{
    auth::hash::hash,
    auth::PostgresStore,
    auth::{models::RegisterForm, store::UserStore},
    DOMAIN,
};
use askama_warp::Template;
use deadpool_postgres::Client as DbClient;
use pasta6_core::Error::DbQueryError;
use pasta6_core::{BaseUser, TemplateContext, User};
use tokio_postgres::Transaction;
use warp::{http::header, hyper::Uri, redirect, reject::custom, reply::with_header};

#[derive(Template)]
#[template(path = "register.html")]
struct RegisterTemplate {
    ctx: TemplateContext<BaseUser>,
}

pub(crate) async fn get_register(
    current_user: Option<BaseUser>,
) -> Result<impl warp::Reply, warp::Rejection> {
    Ok(RegisterTemplate {
        ctx: TemplateContext::new(current_user, DOMAIN.to_owned()),
    })
}

pub(crate) async fn post_register(
    form: RegisterForm,
    mut db: DbClient,
) -> Result<impl warp::Reply, warp::Rejection> {
    let transaction = db
        .transaction()
        .await
        .map_err(DbQueryError)
        .map_err(|e| custom(e))?;
    let store = PostgresStore::<Transaction>::new(&*transaction);
    // TODO: perform proper validation to ensure these aren't empty values and enforce limits
    // on them (e.g. username length).
    // TODO: perform the validation in middleware.
    let user = store
        .create_user(form.username(), &hash(form.password()))
        .await
        .map_err(|e| custom(e))?;

    let session = generate_random_session();
    store
        .set_session(&user, &session)
        .await
        .map_err(|e| custom(e))?;

    transaction
        .commit()
        .await
        .map_err(DbQueryError)
        .map_err(|e| custom(e))?;
    // TODO: should I be using serde_json to serialize the cookie or something like percent
    // encoding?
    let session_cookie = set_session(&serde_json::to_string(&session).unwrap());
    let redirect_uri = Uri::from_static("/");
    Ok(redirect(redirect_uri)).map(|reply| with_header(reply, header::SET_COOKIE, session_cookie))
}
