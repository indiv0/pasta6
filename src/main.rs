// TODO: if the database restarts, we should either reconnect or restart as well.
use std::sync::Arc;
use warp::Filter;

mod db {
    use crate::error::Error;
    use crate::models::{Paste, PasteRequest};
    use std::sync::Arc;
    use tokio_postgres::Client as DbClient;

    const TABLE: &str = "paste";
    const SELECT_FIELDS: &str = "id, created_at, data";

    pub async fn create_db_connection() -> Result<DbClient, tokio_postgres::Error>{
        // Connect to the database.
        let (client, conn) = tokio_postgres::connect("host=localhost user=pastaaaaaa password=pastaaaaaa", tokio_postgres::NoTls).await?;

        // The connection object performs the communication with the database,
        // so spawn it off to run on its own.
        tokio::spawn(async move {
            if let Err(e) = conn.await {
                eprintln!("connection error: {}", e);
            }
        });

        Ok(client)
    }

    pub async fn init_db(client: &DbClient) -> Result<(), tokio_postgres::Error> {
        const INIT_SQL: &str = r#"CREATE TABLE IF NOT EXISTS paste
    (
        id SERIAL PRIMARY KEY NOT NULL,
        created_at timestamp with time zone DEFAULT (now() at time zone 'utc'),
        data bytea
    )"#;

        let _rows = client
            .query(INIT_SQL, &[])
            .await?;

        Ok(())
    }

    fn row_to_paste(row: &tokio_postgres::row::Row) -> Paste {
        let id = row.get(0);
        let created_at = row.get(1);
        let data = row.get(2);
        Paste::new(id, created_at, data)
    }

    pub async fn create_paste(client: Arc<DbClient>, body: PasteRequest) -> Result<Paste, Error> {
        let query = format!("INSERT INTO {} (data) VALUES ($1) RETURNING *", TABLE);
        let row = client
            .query_one(query.as_str(), &[&&body[..]])
            .await
            .map_err(Error::DbQueryError)?;
        Ok(row_to_paste(&row))
    }

    pub async fn get_paste(client: Arc<DbClient>, id: i32) -> Result<Paste, Error> {
        let query = format!("SELECT {} FROM {} WHERE id=$1", SELECT_FIELDS, TABLE);
        let row = client
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
    use std::convert::Infallible;
    use std::sync::Arc;
    use tokio_postgres::Client as DbClient;
    use warp::Filter;
    use warp::http::StatusCode;

    pub async fn health_handler(client: Arc<DbClient>) -> Result<impl warp::Reply, warp::Rejection> {
        // Check if our connection to the DB is still OK.
        client
            .query("SELECT 1", &[])
            .await
            .map_err(|e| warp::reject::custom(Error::DbQueryError(e)))?;

        Ok(StatusCode::OK)
    }

    pub async fn create_paste_handler(body: PasteRequest, client: Arc<DbClient>) -> Result<impl warp::Reply, warp::Rejection> {
        Ok(warp::reply::json(&PasteCreateResponse::of(
            db::create_paste(client, body)
                .await
                .map_err(|e| warp::reject::custom(e))?
        )))
    }

    pub async fn get_paste_handler(id: i32, client: Arc<DbClient>) -> Result<impl warp::Reply, warp::Rejection> {
        Ok(models::paste_to_paste_get_response(
            db::get_paste(client, id)
                .await
                .map_err(|e| warp::reject::custom(e))?
        ))
    }

    pub fn with_db(client: Arc<DbClient>) -> impl Filter<Extract = (Arc<DbClient>,), Error = Infallible> + Clone {
        warp::any().map(move || client.clone())
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
                },
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
    #[derive(Debug)]
    pub enum Error {
        DbQueryError(tokio_postgres::Error),
    }

    impl warp::reject::Reject for Error {}
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
            Self { id, _created_at: created_at, data }
        }
    }

    #[derive(Serialize)]
    pub struct PasteCreateResponse {
        id: i32,
    }

    impl PasteCreateResponse {
        pub fn of(paste: Paste) -> Self {
            Self {
                id: paste.id,
            }
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
    let client = Arc::new(db::create_db_connection().await.expect("create connection error"));

    db::init_db(&client).await.expect("initialize database error");

    let health = warp::path!("health")
        .and(filter::with_db(client.clone()))
        .and_then(filter::health_handler);

    let paste = warp::path("paste");
    let paste = paste
            .and(warp::get())
            .and(warp::path::param())
            .and(filter::with_db(client.clone()))
            .and_then(filter::get_paste_handler)
        .or(paste
            .and(warp::post())
            // Only accept bodies smaller than 16kb
            .and(warp::body::content_length_limit(1024 * 16))
            .and(warp::body::bytes())
            .and(filter::with_db(client))
            .and_then(filter::create_paste_handler));

    let routes = health
        .or(paste)
        .recover(filter::handle_rejection);

    warp::serve(routes)
        .run(([127, 0, 0, 1], 3030))
        .await;

    Ok(())
}
