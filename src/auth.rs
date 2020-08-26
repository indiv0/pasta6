pub use routes::routes;

mod filter {
    use askama_warp::Template;

    #[derive(Template)]
    #[template(path = "register.html")]
    struct RegisterTemplate;

    pub async fn get_register() -> Result<impl warp::Reply, warp::Rejection> {
        Ok(RegisterTemplate)
    }
}

mod routes {
    use crate::auth::filter;
    use warp::Filter;

    /// GET /register
    fn get_register() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path("register")
            .and(warp::get())
            .and_then(filter::get_register)
    }

    pub fn routes() -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        get_register()
    }
}
