use pasta6_core::{get_db_connection, optional_user, with_db, form_body, CoreUserStore, init_server2};
use warp::{path::end, Filter, get, post};
use filter::{health, index, handle_rejection};
use auth::{post_register, get_register, PostgresStore, get_profile};
use std::net::TcpListener;
use deadpool_postgres::Pool;

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
            .and(optional_user::<PostgresStore>(pool.clone()))
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
            .and(optional_user::<PostgresStore>(pool.clone()))
            .and_then(get_profile))
        .recover(handle_rejection);

    init_server2(listener, routes).await.expect("server error")
}
