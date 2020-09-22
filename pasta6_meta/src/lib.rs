use auth::{
    get_login, get_logout, get_profile, get_register, post_login, post_register, PostgresStore,
};
use deadpool_postgres::Pool;
use filter::{handle_rejection, health, index};
use pasta6_core::{
    form_body, get_db_connection, init_server2, optional_session, optional_user, with_db,
    CoreUserStore, TemplateContext, CONFIG,
};
use std::net::TcpListener;
use tokio_postgres::Client;
use warp::{get, path::end, post, Filter};

// TODO: if the database restarts, we should either reconnect or restart as well.
mod auth;
mod filter;

pub async fn run(listener: TcpListener, pool: Pool) {
    let conn = get_db_connection(&pool)
        .await
        .expect("get db connection error");
    auth::init_db(&conn)
        .await
        .expect("initialize database error");

    let routes =
        // GET /
        end()
            .and(get())
            .and(optional_user::<PostgresStore<Client>>(pool.clone()))
            .map(|u| TemplateContext::new(u))
            .and_then(index)
        // GET /health
        .or(warp::path("health")
            .and(get())
            .and(with_db(pool.clone()))
            .and_then(health))
        // GET /register
        .or(warp::path("register")
            .and(get())
            .and(optional_user::<CoreUserStore>(pool.clone()))
            .and_then(get_register))
        // POST /register
        .or(warp::path("register")
            .and(post())
            // TODO: if we submit a malformed form (e.g. no `input` with `name="username"` then on the console we see:
            //
            //     body deserialize error: BodyDeserializeError { cause: Error { err: "missing field `username`" } }
            //
            //  The JSON response is just `{"message": "Invalid body"}`. We should probably take
            //  users to a 4xx page or display a proper error on the website in this scenario.
            .and(form_body())
            .and(with_db(pool.clone()))
            .and_then(post_register))
        // GET /profile
        .or(warp::path("profile")
            .and(get())
            .and(optional_user::<PostgresStore<Client>>(pool.clone()))
            .and_then(get_profile))
        // GET /logout
        .or(warp::path("logout")
            .and(get())
            .and(optional_session())
            .and(with_db(pool.clone()))
            .and_then(get_logout))
        // GET /login
        .or(warp::path("login")
            .and(get())
            .map(|| TemplateContext::new(None))
            .and_then(get_login))
        // POST /login
        .or(warp::path("login")
            .and(post())
            // TODO: if we submit a malformed form (e.g. no `input` with `name="username"` then on the console we see:
            //
            //     body deserialize error: BodyDeserializeError { cause: Error { err: "missing field `username`" } }
            //
            //  The JSON response is just `{"message": "Invalid body"}`. We should probably take
            //  users to a 4xx page or display a proper error on the website in this scenario.
            .and(form_body())
            .and(with_db(pool.clone()))
            .and_then(post_login))
        .recover(handle_rejection);

    init_server2(&CONFIG, listener, routes)
        .await
        .expect("server error")
}
