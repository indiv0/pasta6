use auth::{MetaAuthProvider, MetaUser};
use deadpool_postgres::{Client, Pool};
use filter::{
    get_login, get_logout, get_profile, get_register, handle_rejection, health, index, post_login,
    post_register,
};
use pasta6_core::{
    form_body, get_db_connection, init_server2, with_db, with_token, AuthProvider, ServerConfig,
    TemplateContext, Token, CONFIG,
};
use std::{convert::Infallible, net::TcpListener};
use tracing::error;
use warp::{Filter, get, path::{path, end}, post};

// TODO: if the database restarts, we should either reconnect or restart as well.
mod auth;
mod filter;
mod hash;

async fn init_db(client: &Client) -> Result<(), tokio_postgres::Error> {
    const INIT_SQL: &str = r#"CREATE TABLE IF NOT EXISTS "user"
(
    id SERIAL PRIMARY KEY NOT NULL,
    created_at timestamp with time zone NOT NULL DEFAULT (now() at time zone 'utc'),
    username TEXT UNIQUE NOT NULL CHECK(length(username) <= 15),
    password TEXT NOT NULL CHECK(length(password) <= 128)
)"#;

    let _rows = client.query(INIT_SQL, &[]).await?;

    Ok(())
}

pub async fn run(config: ServerConfig, listener: TcpListener, pool: Pool) {
    let conn = get_db_connection(&pool)
        .await
        .expect("get db connection error");
    init_db(&conn).await.expect("initialize database error");

    let secret_key = config.secret_key().clone();
    let secret_key_1 = config.secret_key().clone();
    let routes =
        // GET /
        end()
            .and(get())
            .and(with_token(config.secret_key().clone(), config.ttl()))
            .and(with_db(pool.clone()))
            .and_then(move |maybe_token: Option<Token>, client: deadpool_postgres::Client| async move {
                Ok::<_, Infallible>(match maybe_token {
                    None => None,
                    // FIXME: remove this unwrap
                    Some(token) => MetaAuthProvider::get_user(&**client, &token).await.unwrap()
                })
            })
            .map(|u: Option<MetaUser>| TemplateContext::new(u))
            .and_then(index)
        // GET /health
        .or(path("health")
            .and(end())
            .and(get())
            .and(with_db(pool.clone()))
            .and_then(health))
        // GET /register
        .or(path("register")
            .and(end())
            .and(get())
            .and(with_token(config.secret_key().clone(), config.ttl()))
            .and(with_db(pool.clone()))
            .and_then(move |maybe_token: Option<Token>, client: deadpool_postgres::Client| async move {
                Ok::<_, Infallible>(match maybe_token {
                    None => None,
                    // FIXME: remove this unwrap
                    Some(token) => MetaAuthProvider::get_user(&**client, &token).await.unwrap()
                })
            })
            .and_then(get_register))
        // POST /register
        .or(path("register")
            .and(post())
            // TODO: if we submit a malformed form (e.g. no `input` with `name="username"` then on the console we see:
            //
            //     body deserialize error: BodyDeserializeError { cause: Error { err: "missing field `username`" } }
            //
            //  The JSON response is just `{"message": "Invalid body"}`. We should probably take
            //  users to a 4xx page or display a proper error on the website in this scenario.
            .and(form_body())
            .and(with_db(pool.clone()))
            .map(move |form, client| (form, client, secret_key.clone()))
            .untuple_one()
            .and_then(post_register))
        // GET /profile
        .or(path("profile")
            .and(end())
            .and(get())
            .and(with_token(config.secret_key().clone(), config.ttl()))
            .and(with_db(pool.clone()))
            .and_then(move |maybe_token: Option<Token>, client: deadpool_postgres::Client| async move {
                error!("maybe_token: {:?}", maybe_token);
                Ok::<_, Infallible>(match maybe_token {
                    None => None,
                    // FIXME: remove this unwrap
                    Some(token) => MetaAuthProvider::get_user(&**client, &token).await.unwrap()
                })
            })
            .and_then(get_profile))
        // GET /logout
        .or(path("logout")
            .and(end())
            .and(get())
            .and(with_db(pool.clone()))
            .and_then(get_logout))
        // GET /login
        .or(path("login")
            .and(end())
            .and(get())
            .map(|| TemplateContext::new(None))
            .and_then(get_login))
        // POST /login
        .or(path("login")
            .and(end())
            .and(post())
            // TODO: if we submit a malformed form (e.g. no `input` with `name="username"` then on the console we see:
            //
            //     body deserialize error: BodyDeserializeError { cause: Error { err: "missing field `username`" } }
            //
            //  The JSON response is just `{"message": "Invalid body"}`. We should probably take
            //  users to a 4xx page or display a proper error on the website in this scenario.
            .and(form_body())
            .and(with_db(pool.clone()))
            .and(warp::any().map(move || secret_key_1.clone()))
            .and_then(post_login))
        .recover(handle_rejection);

    init_server2(&CONFIG, listener, routes)
        .await
        .expect("server error")
}
