// TODO: if the database restarts, we should either reconnect or restart as well.
mod paste;

mod db {
    use crate::error::Error;
    use deadpool_postgres::Client as DbClient;
    use deadpool_postgres::Pool as DbPool;
    use std::env;

    pub fn create_db_pool() -> Result<DbPool, deadpool_postgres::config::ConfigError> {
        use deadpool_postgres::{Config, ManagerConfig, RecyclingMethod};
        let mut cfg = Config::new();
        cfg.host = Some(env::var("PG_HOST").expect("PG_HOST unset"));
        cfg.user = Some(env::var("PG_USER").expect("PG_USER unset"));
        cfg.password = Some(env::var("PG_PASSWORD").expect("PG_PASSWORD unset"));
        cfg.dbname = Some("pastaaaaaa".to_string());
        cfg.manager = Some(ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        });
        cfg.create_pool(tokio_postgres::NoTls)
    }

    pub async fn get_db_connection(pool: &DbPool) -> Result<DbClient, Error> {
        pool.get().await.map_err(Error::DbPoolError)
    }
}

mod filter {
    use askama_warp::Template;
    use crate::db;
    use crate::error::Error;
    use crate::models::ErrorResponse;
    use deadpool_postgres::Client as DbClient;
    use deadpool_postgres::Pool as DbPool;
    use std::convert::Infallible;
    use warp::http::StatusCode;
    use warp::Filter;

    #[derive(Template)]
    #[template(path = "index.html")]
    struct IndexTemplate;

    type Reply<T> = Result<T, warp::Rejection>;
    type InfallibleReply<T> = Result<T, Infallible>;

    pub async fn index() -> InfallibleReply<impl warp::Reply> {
        Ok(IndexTemplate)
    }

    pub async fn health(db: DbClient) -> Reply<impl warp::Reply> {
        // Check if our connection to the DB is still OK.
        db.query("SELECT 1", &[])
            .await
            .map_err(|e| warp::reject::custom(Error::DbQueryError(e)))?;

        Ok(StatusCode::OK)
    }

    pub fn with_db(pool: DbPool) -> impl Filter<Extract = (DbClient,), Error = warp::Rejection> + Clone {
        warp::any().and_then(move || {
            let pool = pool.clone();
            async move {
                db::get_db_connection(&pool)
                    .await
                    .map_err(|e| warp::reject::custom(e))
            }
        })
    }

    pub async fn handle_rejection(err: warp::Rejection) -> InfallibleReply<impl warp::Reply> {
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
                Error::DbQueryError(_) => {
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
    use crate::filter::{with_db, index, health};
    use deadpool_postgres::Pool as DbPool;
    use warp::Filter;

    /// GET /
    fn get_index() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path::end()
            .and(warp::get())
            .and_then(index)
    }

    /// GET /health
    fn get_health(pool: DbPool) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path("health")
            .and(warp::get())
            .and(with_db(pool))
            .and_then(health)
    }

    pub fn routes(pool: DbPool) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        get_index()
            .or(get_health(pool))
    }
}

mod error {
    use std::fmt;

    #[derive(Debug)]
    pub enum Error {
        DbPoolError(deadpool_postgres::PoolError),
        DbQueryError(tokio_postgres::Error),
    }

    impl warp::reject::Reject for Error {}

    impl From<deadpool_postgres::PoolError> for Error {
        fn from(err: deadpool_postgres::PoolError) -> Self {
            Self::DbPoolError(err)
        }
    }

    impl fmt::Display for Error {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
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
    use chrono::{DateTime, Utc};
    use serde_derive::{Deserialize, Serialize};

    #[derive(Deserialize)]
    #[serde(transparent)]
    pub struct Form<T>(T);

    #[derive(Deserialize)]
    pub struct PasteForm {
        data: String,
    }

    impl PasteForm {
        pub fn data(&self) -> &[u8] {
            self.data.as_bytes()
        }
    }

    #[derive(Debug)]
    pub struct Paste {
        id: i32,
        created_at: DateTime<Utc>,
        data: Vec<u8>,
    }

    impl Paste {
        pub fn new(id: i32, created_at: DateTime<Utc>, data: Vec<u8>) -> Self {
            Self {
                id,
                created_at: created_at,
                data,
            }
        }

        pub fn id(&self) -> &i32 {
            &self.id
        }

        pub fn created_at(&self) -> &DateTime<Utc> {
            &self.created_at
        }

        pub fn data(&self) -> &str {
            &std::str::from_utf8(&self.data).unwrap()
        }
    }

    #[derive(Serialize)]
    pub struct PasteCreateResponse {
        id: i32,
    }

    impl PasteCreateResponse {
        // TODO: should this be implemented with `Into` or `From`?
        pub fn of(paste: Paste) -> Self {
            Self { id: paste.id }
        }
    }

    #[derive(Serialize)]
    pub struct ErrorResponse {
        message: String,
    }

    impl ErrorResponse {
        pub fn new(message: String) -> Self {
            Self { message }
        }
    }

    type PasteGetResponse = Vec<u8>;

    pub fn paste_to_paste_get_response(paste: Paste) -> PasteGetResponse {
        paste.data
    }
}

#[tokio::main]
async fn main() -> Result<(), tokio_postgres::Error> {
    use std::env;
    use warp::Filter;

    let pool = db::create_db_pool().expect("create db pool error");

    let conn = db::get_db_connection(&pool)
        .await
        .expect("get db connection error");
    paste::init_db(&conn).await.expect("initialize database error");

    let routes = routes::routes(pool.clone()).or(paste::routes(pool.clone())).recover(filter::handle_rejection);

    let host: std::net::Ipv4Addr = env::var("PASTA6_HOST").expect("PASTA6_HOST unset").parse().unwrap();
    warp::serve(routes).run((host, 3030)).await;

    Ok(())
}
