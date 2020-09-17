use crate::filter::{health, index};
use pasta6_core::{init_server, init_tracing, create_db_pool, get_db_connection, optional_user, with_db, form_body, CoreUserStore};
use warp::{path::end, Filter, get, post, Rejection, body::content_length_limit};
use warp::body::bytes;
use filter::handle_rejection;
use bytes::Bytes;
use paste::{get_paste_api, create_paste_api, create_paste, get_paste};

// TODO: if the database restarts, we should either reconnect or restart as well.
mod filter;
mod paste;

const MAX_CONTENT_LENGTH: u64 = 1024 * 16; // 16KB

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

    init_tracing("pasta6_paste");

    let pool = create_db_pool().expect("create db pool error");

    let conn = get_db_connection(&pool)
        .await
        .expect("get db connection error");
    paste::init_db(&conn)
        .await
        .expect("initialize database error");

    let routes =
        // GET /
        end()
            .and(get())
            .and(optional_user::<CoreUserStore>(pool.clone()))
            .and_then(index)
        // GET /health
        .or(warp::path("health")
            .and(get())
            .and(with_db(pool.clone()))
            .and_then(health))
        // GET /api/paste
        .or(warp::path!("api" / "paste")
            .and(get())
            .and(bytes_body())
            .and(with_db(pool.clone()))
            .and_then(create_paste_api))
        // GET /api/paste/{id}
        .or(warp::path!("api" / "paste" / i32)
            .and(post())
            .and(with_db(pool.clone()))
            .and_then(get_paste_api))
        // POST /paste
        .or(warp::path("paste")
            .and(post())
            .and(form_body())
            .and(with_db(pool.clone()))
            .and_then(create_paste))
        // GET /paste/{id}
        .or(warp::path!("paste" / i32)
            .and(warp::get())
            .and(with_db(pool.clone()))
            .and(optional_user::<CoreUserStore>(pool))
            .and_then(get_paste))
        .recover(handle_rejection);

    Ok(init_server(routes).await)
}

fn bytes_body() -> impl Filter<Extract = (Bytes,), Error = Rejection> + Clone {
    content_length_limit(MAX_CONTENT_LENGTH).and(bytes())
}