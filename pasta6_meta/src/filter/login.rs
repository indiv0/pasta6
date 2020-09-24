use askama_warp::Template;
use deadpool_postgres::Client;
use pasta6_core::{Context, CoreUser, SecretKey, TemplateContext, Token, User};
use pasta6_core::{
    Error::{DbQueryError, SerdeJsonError},
    UserStore,
};
use serde::Deserialize;
use warp::{
    http::header, hyper::Uri, redirect, reject::custom, reply::with_header, Rejection, Reply,
};

use crate::auth::{verify_password, MetaUserStore};

use super::set_session;

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate {
    ctx: TemplateContext<CoreUser>,
}

pub(crate) async fn get_login(ctx: TemplateContext<CoreUser>) -> Result<impl Reply, Rejection> {
    Ok(LoginTemplate { ctx })
}

#[derive(Deserialize)]
pub(crate) struct LoginForm {
    username: String,
    password: String,
}

impl LoginForm {
    pub(crate) fn username(&self) -> &str {
        &self.username
    }

    pub(crate) fn password(&self) -> &str {
        &self.password
    }
}

pub(crate) async fn post_login(
    form: LoginForm,
    client: Client,
    secret_key: SecretKey,
) -> Result<impl Reply, Rejection> {
    // TODO: ensure that we can't reach this page if the session is already set.

    // TODO: perform proper validation to ensure these aren't empty values and enforce limits
    // on them (e.g. username length).
    // TODO: perform the validation in middleware.
    let user = MetaUserStore::get_user_by_username(&**client, form.username())
        .await
        .map_err(DbQueryError)
        .map_err(|e| custom(e))?;

    if let None = user {
        // TODO: display an error to the user if a user with that username was not found, instead.
        todo!();
    }
    let user = user.unwrap();

    if !verify_password(&user, form.password()) {
        // TODO: display an error that the password was incorrect.
        todo!();
    }

    let token = Token::encode(&user, &secret_key)
        .map_err(SerdeJsonError)
        .map_err(|e| custom(e))?;
    let session_cookie = set_session(&token);

    // TODO: redirect to the page they originally wanted to visit.
    let redirect_uri = Uri::from_static("/profile");
    Ok(redirect(redirect_uri)).map(|reply| with_header(reply, header::SET_COOKIE, session_cookie))
}
