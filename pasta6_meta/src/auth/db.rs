use super::store::{UserStore, PostgresStore};
use super::models::RegisterForm;
// TODO: remove this alias
use super::store::UserStruct;
use deadpool_postgres::Client as DbClient;
use pasta6_core::{Error, Session};

pub(crate) async fn init_db(client: &DbClient) -> Result<(), tokio_postgres::Error> {
    const INIT_SQL: &str = r#"CREATE TABLE IF NOT EXISTS p6_user
(
    id SERIAL PRIMARY KEY NOT NULL,
    created_at timestamp with time zone DEFAULT (now() at time zone 'utc'),
    username VARCHAR(15) UNIQUE NOT NULL,
    password VARCHAR(15) NOT NULL,
    session VARCHAR(255) UNIQUE
)"#;

    let _rows = client.query(INIT_SQL, &[]).await?;

    Ok(())
}

pub(crate) async fn create_user(db: &DbClient, form: &RegisterForm) -> Result<UserStruct, Error> {
    let store = PostgresStore::new(db);
    store.create_user(form).await
}

pub(crate) async fn set_session(db: &DbClient, user: &UserStruct, session: &Session) -> Result<(), Error> {
    let store = PostgresStore::new(db);
    store.set_session(user, session).await
}
