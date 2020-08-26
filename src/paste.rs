pub use db::init_db;
pub use routes::routes;

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

    type PasteGetResponse = Vec<u8>;

    pub fn paste_to_paste_get_response(paste: Paste) -> PasteGetResponse {
        paste.data
    }
}

mod db {
    use crate::error::Error;
    use crate::paste::models::Paste;
    use deadpool_postgres::Client as DbClient;

    const TABLE: &str = "paste";
    const SELECT_FIELDS: &str = "id, created_at, data";

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

    pub async fn create_paste(db: &DbClient, body: &[u8]) -> Result<Paste, Error> {
        // TODO: use a prepared statement.
        let query = format!("INSERT INTO {} (data) VALUES ($1) RETURNING *", TABLE);
        let row = db
            .query_one(query.as_str(), &[&body])
            .await
            .map_err(Error::DbQueryError)?;
        Ok(row_to_paste(&row))
    }

    pub async fn get_paste(db: &DbClient, id: i32) -> Result<Paste, Error> {
        let query = format!("SELECT {} FROM {} WHERE id=$1", SELECT_FIELDS, TABLE);
        let row = db
            .query_one(query.as_str(), &[&id])
            .await
            .map_err(Error::DbQueryError)?;
        Ok(row_to_paste(&row))
    }
}

mod filter {
    use askama_warp::Template;
    use crate::paste::db;
    use crate::paste::models::{self, Paste, PasteCreateResponse, PasteForm};
    use deadpool_postgres::Client as DbClient;
    use std::str::FromStr;
    use warp::http::Uri;

    pub async fn create_paste_api(
        body: bytes::Bytes,
        db: DbClient,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        Ok(warp::reply::json(&PasteCreateResponse::of(
            db::create_paste(&db, &body[..])
                .await
                .map_err(|e| warp::reject::custom(e))?,
        )))
    }

    pub async fn get_paste_api(
        id: i32,
        db: DbClient,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        Ok(models::paste_to_paste_get_response(
            db::get_paste(&db, id)
                .await
                .map_err(|e| warp::reject::custom(e))?,
        ))
    }

    pub async fn create_paste(
        body: PasteForm,
        db: DbClient,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        let paste = db::create_paste(&db, body.data())
            .await
            .map_err(|e| warp::reject::custom(e))?;
        assert_eq!(paste.data().as_bytes(), body.data());
        let redirect_uri = Uri::from_str(&format!("/paste/{id}", id=paste.id())).unwrap();
        // TODO: 302 instead of 301 here
        Ok(warp::redirect::redirect(redirect_uri))
    }

    #[derive(Template)]
    #[template(path = "paste.html")]
    struct PasteTemplate {
        _paste: Paste,
    }

    pub async fn get_paste(
        id: i32,
        db: DbClient,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        let paste = db::get_paste(&db, id)
            .await
            .map_err(|e| warp::reject::custom(e))?;
        Ok(PasteTemplate { _paste: paste })
    }
}

mod routes {
    use crate::filter::with_db;
    use crate::paste::filter;
    use deadpool_postgres::Pool as DbPool;
    use serde::de::DeserializeOwned;
    use warp::Filter;

    fn bytes_body() -> impl Filter<Extract = (bytes::Bytes,), Error = warp::Rejection> + Clone {
        warp::body::content_length_limit(1024 * 16)
            .and(warp::body::bytes())
    }

    fn form_body<T>() -> impl Filter<Extract = (T,), Error = warp::Rejection> + Clone
        where T: Send + DeserializeOwned
    {
        warp::body::content_length_limit(1024 * 16)
            .and(warp::body::form())
    }

    /// GET /api/paste
    fn get_paste_api(pool: DbPool) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path("paste")
            .and(warp::get())
            .and(bytes_body())
            .and(with_db(pool))
            .and_then(filter::create_paste_api)
    }

    /// GET /api/paste/{id}
    fn create_paste_api(pool: DbPool) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("api" / "paste" / i32)
            .and(warp::post())
            .and(with_db(pool))
            .and_then(filter::get_paste_api)
    }

    /// POST /paste
    fn create_paste(pool: DbPool) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path("paste")
            .and(warp::post())
            .and(form_body())
            .and(with_db(pool))
            .and_then(filter::create_paste)
    }

    /// GET /paste/{id}
    fn get_paste(pool: DbPool) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path!("paste" / i32)
            .and(warp::get())
            .and(with_db(pool))
            .and_then(filter::get_paste)
    }

    pub fn routes(pool: DbPool) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        get_paste(pool.clone())
            .or(create_paste(pool.clone()))
            .or(get_paste_api(pool.clone()))
            .or(create_paste_api(pool))
    }
}
