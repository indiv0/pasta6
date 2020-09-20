use super::{generate_random_session, set_session};
use crate::auth::{
    models::LoginForm,
    store::{verify_password, UserStore},
    PostgresStore,
};
use askama_warp::Template;
use deadpool_postgres::Client as DbClient;
use pasta6_core::{BaseUser, Config, CoreConfig, TemplateContext, User};
use tokio_postgres::Client;
use warp::{http::header, hyper::Uri, redirect, reply::with_header};

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate<'a> {
    ctx: TemplateContext<'a, CoreConfig, BaseUser>,
}

pub(crate) async fn get_login(
    ctx: TemplateContext<'static, CoreConfig, BaseUser>,
) -> Result<impl warp::Reply, warp::Rejection> {
    Ok(LoginTemplate { ctx })
}

pub(crate) async fn post_login(
    form: LoginForm,
    db: DbClient,
) -> Result<impl warp::Reply, warp::Rejection> {
    // TODO: ensure that we can't reach this page if the session is already set.

    // TODO: perform proper validation to ensure these aren't empty values and enforce limits
    // on them (e.g. username length).
    // TODO: perform the validation in middleware.
    let store = PostgresStore::<Client>::new(&**db);
    let user = store
        .get_user_by_username(form.username())
        .await
        .map_err(|e| warp::reject::custom(e))?;

    if let None = user {
        // TODO: display an error to the user if a user with that username was not found, instead.
        todo!();
    }
    let user = user.unwrap();

    if !verify_password(&user, form.password()) {
        // TODO: display an error that the password was incorrect.
        todo!();
    }

    // TODO: redirect to the page they originally wanted to visit.
    let redirect_uri = Uri::from_static("/profile");
    let session = generate_random_session();
    store
        .set_session(&user, &session)
        .await
        .map_err(|e| warp::reject::custom(e))?;
    // TODO: should I be using serde_json to serialize the cookie or something like percent
    // encoding?
    let session_cookie = set_session(&serde_json::to_string(&session).unwrap());
    Ok(redirect(redirect_uri)).map(|reply| with_header(reply, header::SET_COOKIE, session_cookie))
}
