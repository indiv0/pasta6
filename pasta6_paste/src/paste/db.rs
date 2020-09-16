use crate::paste::models::Paste;
use deadpool_postgres::Client as DbClient;
use pasta6_core::Error;

const TABLE: &str = "paste";
const SELECT_FIELDS: &str = "id, created_at, data";

pub(crate) async fn init_db(client: &DbClient) -> Result<(), tokio_postgres::Error> {
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

pub(crate) async fn create_paste(db: &DbClient, body: &[u8]) -> Result<Paste, Error> {
    // TODO: use a prepared statement.
    let query = format!("INSERT INTO {} (data) VALUES ($1) RETURNING *", TABLE);
    let row = db
        .query_one(query.as_str(), &[&body])
        .await
        .map_err(Error::DbQueryError)?;
    Ok(row_to_paste(&row))
}

pub(crate) async fn get_paste(db: &DbClient, id: i32) -> Result<Paste, Error> {
    let query = format!("SELECT {} FROM {} WHERE id=$1", SELECT_FIELDS, TABLE);
    let row = db
        .query_one(query.as_str(), &[&id])
        .await
        .map_err(Error::DbQueryError)?;
    Ok(row_to_paste(&row))
}
