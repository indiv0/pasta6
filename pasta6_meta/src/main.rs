use pasta6_core::{get_db_connection, init_tracing, init_server, create_db_pool, optional_user, with_db, form_body, CoreUserStore};
use warp::{path::end, Filter, get, post};
use filter::{health, index, handle_rejection};
use auth::{post_register, get_register, PostgresStore, get_profile};

// TODO: if the database restarts, we should either reconnect or restart as well.
mod auth;
mod filter;

/// # Autoreload
/// Install `systemfd` and `cargo-watch`:
/// ```
/// cargo install systemfd cargo-watch
/// ```
/// And run with:
/// ```
/// systemfd --no-pid -s http::0.0.0.0:3030 -- cargo watch -x run
/// ```
#[tokio::main]
async fn main() -> Result<(), tokio_postgres::Error> {
    main_inner().await
}

// `main_inner` is a separate function from `main` because rust doesn't provide
// helpful messages for errors originating in a method annotated with `#[tokio::main]`.
async fn main_inner() -> Result<(), tokio_postgres::Error> {
    better_panic::install();

    init_tracing("pasta6_meta");

    let pool = create_db_pool().expect("create db pool error");

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
            .and(optional_user::<PostgresStore>(pool))
            .and_then(get_profile))
        .recover(handle_rejection);

    Ok(init_server(routes).await)
}