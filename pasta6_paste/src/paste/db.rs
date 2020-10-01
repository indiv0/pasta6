use crate::paste::models::Paste;
use deadpool_postgres::Client;

macro_rules! paste_table {
    () => {
        "paste"
    };
}

pub(crate) async fn init_db(client: &Client) -> Result<(), tokio_postgres::Error> {
    const INIT_SQL: [&str; 2] = [
        r#"
        CREATE TABLE IF NOT EXISTS paste
        (
            id SERIAL PRIMARY KEY NOT NULL,
            created_at timestamp with time zone NOT NULL DEFAULT (now() at time zone 'utc'),
            data bytea NOT NULL
        )"#,
        r#"
        CREATE TABLE IF NOT EXISTS "user"
        (
            id SERIAL PRIMARY KEY NOT NULL,
            created_at timestamp with time zone NOT NULL DEFAULT (now() at time zone 'utc'),
            username TEXT UNIQUE NOT NULL CHECK(length(username) <= 15)
        )
        "#,
    ];

    for query in &INIT_SQL {
        let _rows = client.query(*query, &[]).await?;
    }

    Ok(())
}

pub(crate) async fn create_paste(
    client: &Client,
    body: &[u8],
) -> Result<Paste, tokio_postgres::Error> {
    // TODO: use a prepared statement.
    const QUERY: &str = concat!(
        "INSERT INTO ",
        paste_table!(),
        " (data) VALUES ($1) RETURNING *"
    );
    let row = client.query_one(QUERY, &[&body]).await?;
    Ok(row_to_paste(&row))
}

pub(crate) async fn get_paste(client: &Client, id: i32) -> Result<Paste, tokio_postgres::Error> {
    const QUERY: &str = concat!(
        "SELECT id, created_at, data FROM ",
        paste_table!(),
        " WHERE id = $1"
    );
    let row = client.query_one(QUERY, &[&id]).await?;
    Ok(row_to_paste(&row))
}

fn row_to_paste(row: &tokio_postgres::row::Row) -> Paste {
    let id = row.get(0);
    let created_at = row.get(1);
    let data = row.get(2);
    Paste::new(id, created_at, data)
}
