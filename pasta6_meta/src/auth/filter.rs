use super::db;
use super::models::RegisterForm;
use askama_warp::Template;
use deadpool_postgres::Client as DbClient;
use pasta6_core::{Session, SESSION_COOKIE_NAME, User, TemplateContext};
use rand::Rng;
use warp::http::Uri;

#[derive(Template)]
#[template(path = "register.html")]
struct RegisterTemplate {
    ctx: TemplateContext,
}

pub(crate) async fn get_register(
    current_user: Option<User>,
) -> Result<impl warp::Reply, warp::Rejection> {
    Ok(RegisterTemplate {
        ctx: TemplateContext::new(current_user),
    })
}

pub(crate) async fn post_register(
    form: RegisterForm,
    db: DbClient,
) -> Result<impl warp::Reply, warp::Rejection> {
    // TODO: perform proper validation to ensure these aren't empty values and enforce limits
    // on them (e.g. username length).
    // TODO: perform the validation in middleware.
    let user = db::create_user(&db, &form)
        .await
        .map_err(|e| warp::reject::custom(e))?;
    let redirect_uri = Uri::from_static("/");
    // TODO: generate the session ID in a cryptographically secure way.
    let session_id = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(30)
        .collect();
    let session = Session::new(session_id);
    db::set_session(&db, &user, &session)
        .await
        .map_err(|e| warp::reject::custom(e))?;
    // TODO: should I be using serde_json to serialize the cookie or something like percent
    // encoding?
    let session_cookie = format!(
        "{}={}",
        SESSION_COOKIE_NAME,
        serde_json::to_string(&session).unwrap()
    );
    Ok(warp::redirect::redirect(redirect_uri)).map(|reply| {
        warp::reply::with_header(reply, warp::http::header::SET_COOKIE, session_cookie)
    })
}
