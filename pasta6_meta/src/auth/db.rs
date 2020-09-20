use deadpool_postgres::Client as DbClient;

pub(crate) async fn init_db(client: &DbClient) -> Result<(), tokio_postgres::Error> {
    const INIT_SQL: &str = r#"CREATE TABLE IF NOT EXISTS p6_user
(
    id SERIAL PRIMARY KEY NOT NULL,
    created_at timestamp with time zone DEFAULT (now() at time zone 'utc'),
    username TEXT UNIQUE NOT NULL CHECK(length(username) <= 15),
    password TEXT NOT NULL CHECK(length(password) <= 128),
    session TEXT UNIQUE CHECK(length(session) <= 255)
)"#;

    let _rows = client.query(INIT_SQL, &[]).await?;

    Ok(())
}
