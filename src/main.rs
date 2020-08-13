// TODO: if the database restarts, we should either reconnect or restart as well.
use serde_derive::Serialize;
use std::sync::Arc;
use warp::Filter;

mod db {
    use tokio_postgres::Client as DbClient;

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
}

mod filter {
    use crate::ErrorResponse;
    use crate::error::Error;
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

        let json = warp::reply::json(&ErrorResponse {
            message: message.into(),
        });

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

#[derive(Serialize)]
struct ErrorResponse {
    message: String,
}

#[tokio::main]
async fn main() -> Result<(), tokio_postgres::Error> {
    let client = Arc::new(db::create_db_connection().await.expect("create connection error"));

    db::init_db(&client).await.expect("initialize database error");

    let health = warp::path!("health")
        .and(filter::with_db(client))
        .and_then(filter::health_handler);

    let paste = warp::post()
        .and(warp::path("paste"))
        // Only accept bodies smaller than 16kb
        .and(warp::body::content_length_limit(1024 * 16))
        .and(warp::body::bytes())
        .map(|bytes| {
            format!("bytes = {:?}", bytes)
        });

    let routes = health
        .or(paste)
        .recover(filter::handle_rejection);

    warp::serve(routes)
        .run(([127, 0, 0, 1], 3030))
        .await;

    Ok(())
}
