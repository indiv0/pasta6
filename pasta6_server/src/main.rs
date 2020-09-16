use std::env;
use tracing::info;
use tracing_subscriber::fmt::format::FmtSpan;

// TODO: if the database restarts, we should either reconnect or restart as well.
mod auth;
mod paste;
mod session;

mod db {
    use crate::error::Error;
    use deadpool_postgres::Client as DbClient;
    use deadpool_postgres::Pool as DbPool;
    use std::env;

    pub(crate) fn create_db_pool() -> Result<DbPool, deadpool_postgres::config::ConfigError> {
        use deadpool_postgres::{Config, ManagerConfig, RecyclingMethod};
        let mut cfg = Config::new();
        cfg.host = Some(env::var("POSTGRES_HOST").expect("POSTGRES_HOST unset"));
        cfg.user = Some(env::var("POSTGRES_USER").expect("POSTGRES_USER unset"));
        cfg.password = Some(env::var("POSTGRES_PASSWORD").expect("POSTGRES_PASSWORD unset"));
        cfg.dbname = Some(env::var("POSTGRES_DB").expect("POSTGRES_DB unset"));
        cfg.manager = Some(ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        });
        cfg.create_pool(tokio_postgres::NoTls)
    }

    pub(crate) async fn get_db_connection(pool: &DbPool) -> Result<DbClient, Error> {
        pool.get().await.map_err(Error::DbPoolError)
    }
}

mod filter {
    use super::db;
    use super::models::ErrorResponse;
    use crate::auth::User;
    use crate::error::Error;
    use askama_warp::Template;
    use deadpool_postgres::Client as DbClient;
    use deadpool_postgres::Pool as DbPool;
    use std::convert::Infallible;
    use warp::http::StatusCode;
    use warp::Filter;

    pub(crate) struct TemplateContext {
        current_user: Option<User>,
    }

    impl TemplateContext {
        pub(crate) fn new(current_user: Option<User>) -> Self {
            Self { current_user }
        }

        pub(crate) fn current_user(&self) -> Option<&User> {
            self.current_user.as_ref()
        }
    }

    #[derive(Template)]
    #[template(path = "index.html")]
    struct IndexTemplate {
        ctx: TemplateContext,
    }

    // TODO: only get a DB connection if the session is present.
    pub(crate) async fn index(current_user: Option<User>) -> Result<impl warp::Reply, warp::Rejection> {
        Ok(IndexTemplate {
            ctx: TemplateContext::new(current_user),
        })
    }

    pub(crate) async fn health(db: DbClient) -> Result<impl warp::Reply, warp::Rejection> {
        // Check if our connection to the DB is still OK.
        db.query("SELECT 1", &[])
            .await
            .map_err(|e| warp::reject::custom(Error::DbQueryError(e)))?;

        Ok(StatusCode::OK)
    }

    pub(crate) fn with_db(
        pool: DbPool,
    ) -> impl Filter<Extract = (DbClient,), Error = warp::Rejection> + Clone {
        warp::any().and_then(move || {
            let pool = pool.clone();
            async move {
                db::get_db_connection(&pool)
                    .await
                    .map_err(|e| warp::reject::custom(e))
            }
        })
    }

    pub(crate) async fn handle_rejection(err: warp::Rejection) -> Result<impl warp::Reply, Infallible> {
        let code;
        let message;

        if err.is_not_found() {
            code = StatusCode::NOT_FOUND;
            message = "Not found";
        } else if let Some(e) = err.find::<warp::filters::body::BodyDeserializeError>() {
            // TODO: disable this log line outside of development
            eprintln!("body deserialize error: {:?}", e);
            code = StatusCode::BAD_REQUEST;
            message = "Invalid body";
        } else if let Some(e) = err.find::<Error>() {
            match e {
                Error::DbQueryError(e) => {
                    eprintln!("could not execute request: {:?}", e);
                    code = StatusCode::BAD_REQUEST;
                    message = "Could not execute request";
                }
                _ => {
                    eprintln!("unhandled application error: {:?}", err);
                    code = warp::http::StatusCode::INTERNAL_SERVER_ERROR;
                    message = "Internal server error";
                }
            }
        } else if let Some(_) = err.find::<warp::reject::MethodNotAllowed>() {
            code = StatusCode::METHOD_NOT_ALLOWED;
            message = "Method not allowed";
        } else {
            eprintln!("unhandled error: {:?}", err);
            code = StatusCode::INTERNAL_SERVER_ERROR;
            message = "Internal server error";
        }

        let json = warp::reply::json(&ErrorResponse::new(message.into()));

        Ok(warp::reply::with_status(json, code))
    }
}

mod routes {
    use crate::auth;
    use crate::filter::{health, index, with_db};
    use deadpool_postgres::Pool as DbPool;
    use serde::de::DeserializeOwned;
    use warp::Filter;

    pub(crate) fn form_body<T>() -> impl Filter<Extract = (T,), Error = warp::Rejection> + Clone
    where
        T: Send + DeserializeOwned,
    {
        warp::body::content_length_limit(1024 * 16).and(warp::body::form())
    }

    /// GET /
    fn get_index(
        pool: DbPool,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path::end()
            .and(warp::get())
            .and(auth::optional_user(pool))
            .and_then(index)
    }

    /// GET /health
    fn get_health(
        pool: DbPool,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path("health")
            .and(warp::get())
            .and(with_db(pool))
            .and_then(health)
    }

    pub(crate) fn routes(
        pool: DbPool,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        get_index(pool.clone()).or(get_health(pool))
    }
}

mod error {
    use std::fmt;

    #[derive(Debug)]
    pub(crate) enum Error {
        SerdeJsonError(serde_json::error::Error),
        DbPoolError(deadpool_postgres::PoolError),
        DbQueryError(tokio_postgres::Error),
    }

    impl warp::reject::Reject for Error {}

    impl From<serde_json::error::Error> for Error {
        fn from(err: serde_json::error::Error) -> Self {
            Self::SerdeJsonError(err)
        }
    }

    impl From<deadpool_postgres::PoolError> for Error {
        fn from(err: deadpool_postgres::PoolError) -> Self {
            Self::DbPoolError(err)
        }
    }

    impl fmt::Display for Error {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                &Error::SerdeJsonError(ref e) => {
                    write!(f, "error serializing/deserializing JSON data: {0}", e)
                }
                &Error::DbPoolError(ref e) => {
                    write!(f, "error getting connection from DB pool: {0}", e)
                }
                &Error::DbQueryError(ref e) => write!(f, "error executing DB query: {0}", e),
            }
        }
    }

    impl std::error::Error for Error {}
}

mod models {
    use serde_derive::Serialize;

    #[derive(Serialize)]
    pub(crate) struct ErrorResponse {
        message: String,
    }

    impl ErrorResponse {
        pub(crate) fn new(message: String) -> Self {
            Self { message }
        }
    }
}

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
    use hyper::server::Server;
    use listenfd::ListenFd;
    use std::convert::Infallible;
    use std::env;
    use warp::Filter;

    better_panic::install();

    init_tracing();

    let pool = db::create_db_pool().expect("create db pool error");

    let conn = db::get_db_connection(&pool)
        .await
        .expect("get db connection error");
    auth::init_db(&conn)
        .await
        .expect("initialize database error");
    paste::init_db(&conn)
        .await
        .expect("initialize database error");

    let routes = routes::routes(pool.clone())
        .or(auth::routes(pool.clone()))
        .or(paste::routes(pool.clone()))
        .recover(filter::handle_rejection);

    // Wrap all the routes with a filter that creates a `tracing` span for
    // each request we receive, including data about the request.
    let routes = routes.with(warp::trace::request());

    // hyper lets us build a server from a TcpListener. Thus, we'll need to
    // convert our `warp::Filter` into a `hyper::service::MakeService` for use
    // with a `hyper::server::Server`.
    let svc = warp::service(routes);

    let make_svc = hyper::service::make_service_fn(|_: _| {
        // the cline is there because not all warp filters impl Copy
        let svc = svc.clone();
        async move { Ok::<_, Infallible>(svc) }
    });

    let mut listenfd = ListenFd::from_env();
    // if listenfd doesn't take a TcpListener (i.e. we're not running via the
    // command above), we fall back to explicitly binding to a given host:port.
    let server = if let Some(l) = listenfd.take_tcp_listener(0).unwrap() {
        info!("initializing server with listenfd");
        Server::from_tcp(l).unwrap()
    } else {
        let host: std::net::Ipv4Addr = env::var("PASTA6_HOST")
            .expect("PASTA6_HOST unset")
            .parse()
            .unwrap();
        info!("initializing server on {}:{}", host, 3030);
        Server::bind(&(host, 3030).into())
    };

    server.serve(make_svc).await.unwrap();

    Ok(())
}

fn init_tracing() {
    // Filter traces based on the RUST_LOG env var, or, if it's not set,
    // default to show the output of the example.
    let env_filter = env::var("RUST_LOG").unwrap_or_else(|_| "pasta6_server=trace,tracing=info,warp=debug".to_owned());

    // Configure the default `tracing` subscriber.
    // The `fmt` subscriber from the `tracing-subscriber` crate logs `tracing`
    // events to stdout. Other subscribers are available for integrating with
    // distributed tracing systems such as OpenTelemetry.
    tracing_subscriber::fmt()
        // Use the filter we built above to determine which traces to record.
        .with_env_filter(env_filter)
        // Record an event when each span closes. This can be used to time our
        // routes' durations!
        .with_span_events(FmtSpan::CLOSE)
        .init();
}