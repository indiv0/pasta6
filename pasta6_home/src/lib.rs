use deadpool_postgres::Client;
use deadpool_postgres::Pool;
use filter::{handle_rejection, health, index};
use pasta6_core::{
    get_db_connection, init_server2, with_db, with_token, AuthProvider, CoreAuthProvider,
    ServerConfig, Token, CONFIG,
};
use std::{convert::Infallible, net::TcpListener};
use warp::{get, path, path::end, Filter};

mod filter;

pub const SITE: &str = "home";

// TODO: add an updated_at field for all tables
// TODO: ensure both created_at and updated_at are non-null
// FIXME: we don't actually need the session column for this microservice
async fn init_db(client: &Client) -> Result<(), tokio_postgres::Error> {
    const INIT_SQL: &str = r#"CREATE TABLE IF NOT EXISTS "user"
(
    id SERIAL PRIMARY KEY NOT NULL,
    created_at timestamp with time zone NOT NULL DEFAULT (now() at time zone 'utc'),
    username TEXT UNIQUE NOT NULL CHECK(length(username) <= 15)
)"#;

    let _rows = client.query(INIT_SQL, &[]).await?;

    Ok(())
}

pub async fn run(config: ServerConfig, listener: TcpListener, pool: Pool) {
    let conn = get_db_connection(&pool)
        .await
        .expect("get db connection error");
    init_db(&conn).await.expect("initialize database error");

    let routes =
        // GET /
        end()
            .and(get())
            .and(with_token(config.secret_key().clone(), config.ttl()))
            .and(with_db(pool.clone()))
            .and_then(move |maybe_token: Option<Token>, client: deadpool_postgres::Client| async move {
                Ok::<_, Infallible>(match maybe_token {
                    None => None,
                    // FIXME: remove this unwrap
                    Some(token) => CoreAuthProvider::get_user(&**client, &token).await.unwrap()
                })
            })
            .and_then(index)
        // GET /health
        .or(path("health")
            .and(end())
            .and(get())
            .and(with_db(pool.clone()))
            .and_then(health))
        .recover(handle_rejection);

    init_server2(&*CONFIG, listener, routes)
        .await
        .expect("server error")
}
