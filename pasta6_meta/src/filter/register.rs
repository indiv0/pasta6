use super::set_session;
use crate::{auth::MetaUser, auth::MetaUserStore, hash::hash};
use askama_warp::Template;
use deadpool_postgres::Client;
use pasta6_core::{Context, Error::DbQueryError, Error::SerdeJsonError, SecretKey, Token};
use pasta6_core::{TemplateContext, User};
use serde::Deserialize;
use warp::{
    http::header, hyper::Uri, redirect, reject::custom, reply::with_header, Rejection, Reply,
};

#[derive(Template)]
#[template(path = "register.html")]
struct RegisterTemplate {
    ctx: TemplateContext<MetaUser>,
}

pub(crate) async fn get_register(current_user: Option<MetaUser>) -> Result<impl Reply, Rejection> {
    Ok(RegisterTemplate {
        ctx: TemplateContext::new(current_user),
    })
}

#[derive(Deserialize)]
pub(crate) struct RegisterForm {
    username: String,
    password: String,
}

impl RegisterForm {
    pub(crate) fn username(&self) -> &str {
        &self.username
    }

    pub(crate) fn password(&self) -> &str {
        &self.password
    }
}

pub(crate) async fn post_register(
    form: RegisterForm,
    mut client: Client,
    secret_key: SecretKey,
) -> Result<impl Reply, Rejection> {
    let transaction = client
        .transaction()
        .await
        .map_err(DbQueryError)
        .map_err(|e| custom(e))?;
    // TODO: perform proper validation to ensure these aren't empty values and enforce limits
    // on them (e.g. username length).
    // TODO: perform the validation in middleware.
    let user = MetaUserStore::create_user(&*transaction, form.username(), &hash(form.password()))
        .await
        .map_err(DbQueryError)
        .map_err(|e| custom(e))?;

    let token = Token::encode(&user, &secret_key)
        .map_err(SerdeJsonError)
        .map_err(|e| custom(e))?;
    let session_cookie = set_session(&token);

    transaction
        .commit()
        .await
        .map_err(DbQueryError)
        .map_err(|e| custom(e))?;
    let redirect_uri = Uri::from_static("/");
    Ok(redirect(redirect_uri)).map(|reply| with_header(reply, header::SET_COOKIE, session_cookie))
}
