use pasta6_core::{get_db_connection, init_tracing, init_server, create_db_pool, optional_user, with_db};
use warp::{path::end, Filter, get};
use filter::{health, index, handle_rejection};

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

    init_tracing("pasta6_home");

    let pool = create_db_pool().expect("create db pool error");

    let _conn = get_db_connection(&pool)
        .await
        .expect("get db connection error");

    let routes =
        // GET /
        end()
            .and(get())
            .and(optional_user(pool.clone()))
            .and_then(index)
        // GET /health
        .or(warp::path("health")
            .and(get())
            .and(with_db(pool.clone()))
            .and_then(health))
        .recover(handle_rejection);

    Ok(init_server(routes).await)
}
