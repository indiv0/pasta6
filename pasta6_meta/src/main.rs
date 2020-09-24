use pasta6_core::{bind, create_db_pool, init_tracing, ServerConfig};
use pasta6_meta::run;

const SITE: &str = "meta";

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

    let config = ServerConfig::new();
    let listener = bind();
    let pool = create_db_pool(SITE).expect("create db pool error");
    run(config, listener, pool).await;

    Ok(())
}
