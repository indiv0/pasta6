// TODO: if the database restarts, we should either reconnect or restart as well.
use warp::Filter;

mod db {
    use crate::error::Error;
    use crate::models::{Paste, PasteRequest};
    use deadpool_postgres::Client as DbClient;
    use deadpool_postgres::Pool as DbPool;
    use std::env;

    const TABLE: &str = "paste";
    const SELECT_FIELDS: &str = "id, created_at, data";

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

    pub async fn init_db(client: &DbClient) -> Result<(), tokio_postgres::Error> {
        const INIT_SQL: &str = r#"CREATE TABLE IF NOT EXISTS paste
    (
        id SERIAL PRIMARY KEY NOT NULL,
        created_at timestamp with time zone DEFAULT (now() at time zone 'utc'),
        data bytea
    )"#;

        let _rows = client.query(INIT_SQL, &[]).await?;

        Ok(())
    }

    // TODO: does this belong here or in models?
    fn row_to_paste(row: &tokio_postgres::row::Row) -> Paste {
        let id = row.get(0);
        let created_at = row.get(1);
        let data = row.get(2);
        Paste::new(id, created_at, data)
    }

    pub async fn create_paste(pool: &DbPool, body: PasteRequest) -> Result<Paste, Error> {
        let conn = get_db_connection(pool).await?;
        // TODO: use a prepared statement.
        let query = format!("INSERT INTO {} (data) VALUES ($1) RETURNING *", TABLE);
        let row = conn
            .query_one(query.as_str(), &[&&body[..]])
            .await
            .map_err(Error::DbQueryError)?;
        Ok(row_to_paste(&row))
    }

    pub async fn get_paste(pool: &DbPool, id: i32) -> Result<Paste, Error> {
        let conn = get_db_connection(pool).await?;
        let query = format!("SELECT {} FROM {} WHERE id=$1", SELECT_FIELDS, TABLE);
        let row = conn
            .query_one(query.as_str(), &[&id])
            .await
            .map_err(Error::DbQueryError)?;
        Ok(row_to_paste(&row))
    }
}

mod filter {
    use crate::db;
    use crate::error::Error;
    use crate::models::{self, ErrorResponse, PasteCreateResponse, PasteRequest};
    use deadpool_postgres::Pool as DbPool;
    use std::convert::Infallible;
    use warp::http::StatusCode;
    use warp::Filter;

    pub async fn health_handler(pool: DbPool) -> Result<impl warp::Reply, warp::Rejection> {
        let conn = db::get_db_connection(&pool)
            .await
            .map_err(|e| warp::reject::custom(e))?;
        // Check if our connection to the DB is still OK.
        conn.query("SELECT 1", &[])
            .await
            .map_err(|e| warp::reject::custom(Error::DbQueryError(e)))?;

        Ok(StatusCode::OK)
    }

    pub async fn create_paste_handler(
        body: PasteRequest,
        pool: DbPool,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        Ok(warp::reply::json(&PasteCreateResponse::of(
            db::create_paste(&pool, body)
                .await
                .map_err(|e| warp::reject::custom(e))?,
        )))
    }

    pub async fn get_paste_handler(
        id: i32,
        pool: DbPool,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        Ok(models::paste_to_paste_get_response(
            db::get_paste(&pool, id)
                .await
                .map_err(|e| warp::reject::custom(e))?,
        ))
    }

    pub fn with_db(pool: DbPool) -> impl Filter<Extract = (DbPool,), Error = Infallible> + Clone {
        warp::any().map(move || pool.clone())
    }

    pub async fn handle_rejection(err: warp::Rejection) -> Result<impl warp::Reply, Infallible> {
        let code;
        let message;

        if err.is_not_found() {
            code = StatusCode::NOT_FOUND;
            message = "Not found";
        } else if let Some(_) = err.find::<warp::filters::body::BodyDeserializeError>() {
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
    use serde_derive::Serialize;

    pub type PasteRequest = bytes::Bytes;

    pub struct Paste {
        id: i32,
        _created_at: DateTime<Utc>,
        data: Vec<u8>,
    }

    impl Paste {
        pub fn new(id: i32, created_at: DateTime<Utc>, data: Vec<u8>) -> Self {
            Self {
                id,
                _created_at: created_at,
                data,
            }
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
    let pool = db::create_db_pool().expect("create db pool error");

    let conn = db::get_db_connection(&pool)
        .await
        .expect("get db connection error");
    db::init_db(&conn).await.expect("initialize database error");

    let health = warp::path!("health")
        .and(filter::with_db(pool.clone()))
        .and_then(filter::health_handler);

    let paste = warp::path("paste");
    let paste = paste
        .and(warp::get())
        .and(warp::path::param())
        .and(filter::with_db(pool.clone()))
        .and_then(filter::get_paste_handler)
        .or(paste
            .and(warp::post())
            // Only accept bodies smaller than 16kb
            .and(warp::body::content_length_limit(1024 * 16))
            .and(warp::body::bytes())
            .and(filter::with_db(pool))
            .and_then(filter::create_paste_handler));

    let routes = health.or(paste).recover(filter::handle_rejection);

    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;

    Ok(())
}
