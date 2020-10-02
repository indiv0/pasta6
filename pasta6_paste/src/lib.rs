use crate::filter::{health, index};
use deadpool_postgres::Pool;
use filter::handle_rejection;
use pasta6_core::{
    form_body, get_db_connection, init_server2, with_db, with_token, AuthProvider,
    CoreAuthProvider, ServerConfig, Token, CONFIG,
};
use paste::{Hash, create_paste, get_paste};
use std::{convert::Infallible, net::TcpListener};
use warp::{get, path, path::end, post, Filter};

// TODO: if the database restarts, we should either reconnect or restart as well.
mod filter;
mod paste;

pub async fn run(config: ServerConfig, listener: TcpListener, pool: Pool) {
    let conn = get_db_connection(&pool)
        .await
        .expect("get db connection error");
    paste::init_db(&conn)
        .await
        .expect("initialize database error");

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
            .and(get())
            .and(with_db(pool.clone()))
            .and_then(health))
        // POST /paste
        .or(path("paste")
            .and(end())
            .and(post())
            .and(form_body())
            .and(with_db(pool.clone()))
            .and_then(create_paste))
        // GET /paste/{id}
        .or(path!("paste" / Hash)
            .and(get())
            .and(with_token(config.secret_key().clone(), config.ttl()))
            .and(with_db(pool.clone()))
            .and_then(move |hash: Hash, maybe_token: Option<Token>, client: deadpool_postgres::Client| async move {
                Ok::<_, Infallible>((hash, match maybe_token {
                    None => None,
                    // FIXME: remove this unwrap
                    Some(token) => CoreAuthProvider::get_user(&**client, &token).await.unwrap()
                }))
            })
            .untuple_one()
            .and(with_db(pool.clone()))
            .map(|hash: Hash, maybe_user: Option<_>, client: deadpool_postgres::Client| (hash, client, maybe_user))
            .untuple_one()
            .and_then(get_paste))
        .recover(handle_rejection);

    init_server2(&CONFIG, listener, routes)
        .await
        .expect("server error")
}