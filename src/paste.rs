pub use routes::routes;

mod routes {
    use crate::filter::{self, with_db};
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
