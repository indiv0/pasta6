use crate::filter::{health, index};
use bytes::Bytes;
use deadpool_postgres::Pool;
use filter::handle_rejection;
use pasta6_core::{
    form_body, get_db_connection, init_server2, with_db, with_token, AuthProvider,
    CoreAuthProvider, ServerConfig, Token, CONFIG,
};
use paste::{create_paste, create_paste_api, get_paste, get_paste_api};
use std::{convert::Infallible, net::TcpListener};
use warp::{body::bytes, path};
use warp::{body::content_length_limit, get, path::end, post, Filter, Rejection};

// TODO: if the database restarts, we should either reconnect or restart as well.
mod filter;
mod paste;

const MAX_CONTENT_LENGTH: u64 = 1024 * 16; // 16KB

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
        // GET /api/paste
        .or(path!("api" / "paste")
            .and(get())
            .and(bytes_body())
            .and(with_db(pool.clone()))
            .and_then(create_paste_api))
        // GET /api/paste/{id}
        .or(path!("api" / "paste" / i32)
            .and(post())
            .and(with_db(pool.clone()))
            .and_then(get_paste_api))
        // POST /paste
        .or(path("paste")
            .and(post())
            .and(form_body())
            .and(with_db(pool.clone()))
            .and_then(create_paste))
        // GET /paste/{id}
        .or(path!("paste" / i32)
            .and(get())
            .and(with_token(config.secret_key().clone(), config.ttl()))
            .and(with_db(pool.clone()))
            .and_then(move |id: i32, maybe_token: Option<Token>, client: deadpool_postgres::Client| async move {
                Ok::<_, Infallible>((id, match maybe_token {
                    None => None,
                    // FIXME: remove this unwrap
                    Some(token) => CoreAuthProvider::get_user(&**client, &token).await.unwrap()
                }))
            })
            .untuple_one()
            .and(with_db(pool.clone()))
            .map(|id: i32, maybe_user: Option<_>, client: deadpool_postgres::Client| (id, client, maybe_user))
            .untuple_one()
            .and_then(get_paste))
        .recover(handle_rejection);

    init_server2(&CONFIG, listener, routes)
        .await
        .expect("server error")
}

fn bytes_body() -> impl Filter<Extract = (Bytes,), Error = Rejection> + Clone {
    content_length_limit(MAX_CONTENT_LENGTH).and(bytes())
}
