use deadpool_postgres::Pool;
use filter::{handle_rejection, health, index};
use pasta6_core::{get_db_connection, init_server2, optional_user, with_db, CoreUserStore, CONFIG};
use std::net::TcpListener;
use warp::{get, path::end, Filter};

mod filter;

pub async fn run(listener: TcpListener, pool: Pool) {
    let _conn = get_db_connection(&pool)
        .await
        .expect("get db connection error");

    let routes =
        // GET /
        end()
            .and(get())
            .and(optional_user::<CoreUserStore>(pool.clone()))
            .and_then(index)
        // GET /health
        .or(warp::path("health")
            .and(get())
            .and(with_db(pool.clone()))
            .and_then(health))
        .recover(handle_rejection);

    init_server2(&*CONFIG, listener, routes)
        .await
        .expect("server error")
}
