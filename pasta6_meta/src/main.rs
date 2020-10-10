use pasta6_core::{bind, create_db_pool, init_tracing, Error, ServerConfig, CONFIG};
use pasta6_meta::run;
use sentry::ClientOptions;
use tracing::{error, info};

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
async fn main() {
    better_panic::install();
    init_tracing("pasta6_meta");
    if let Err(ref e) = _main().await {
        error!("{}", e);
        sentry::capture_error(e);
    }
}

// `_main` is a separate function from `main` because rust doesn't provide
// helpful messages for errors originating in a method annotated with `#[tokio::main]`.
async fn _main() -> Result<(), Error> {
    // Load the Sentry SDK client key (AKA the DSN), which we need to specify to be able
    // to send data to Sentry.
    let sentry_dsn = CONFIG.sentry_dsn()?;
    info!("Initializing sentry with DSN: {}", sentry_dsn);
    // Initialize a guard that when freed, will prevent process exist until all events
    // have been sent (within a timeout).
    let _guard = sentry::init((
        sentry_dsn,
        ClientOptions {
            release: Some(env!("GIT_HASH").into()),
            debug: cfg!(debug_assertions),
            environment: if cfg!(debug_assertions) {
                Some("development".into())
            } else {
                Some("production".into())
            },
            ..Default::default()
        },
    ));

    let config = ServerConfig::new();
    let listener = bind();
    let pool = create_db_pool(SITE).expect("create db pool error");
    run(config, listener, pool).await;

    Ok(())
}
