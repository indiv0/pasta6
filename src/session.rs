pub use models::Session;
pub use routes::optional_session;

pub const SESSION_COOKIE_NAME: &str = "session";

mod models {
    use serde::Serialize;

    #[derive(Serialize)]
    pub struct Session(String);

    impl Session {
        pub fn new(session_id: String) -> Self {
            Self(session_id)
        }

        pub fn session_id(&self) -> &str {
            &self.0
        }
    }
}

mod routes {
    use super::SESSION_COOKIE_NAME;
    use super::models::Session;
    use warp::Filter;

    pub fn optional_session() -> impl Filter<Extract = (Option<Session>,), Error = std::convert::Infallible> + Clone {
        warp::filters::cookie::optional(SESSION_COOKIE_NAME)
            .map(|maybe_cookie: Option<String>| {
                if let None = maybe_cookie {
                    return None;
                }

                let maybe_session_id: Option<String> = serde_json::from_str(&maybe_cookie.unwrap()).map_err(|e| eprintln!("failed to deserialize session cookie: {:?}", e)).ok();
                if let None = maybe_session_id {
                    return None;
                }

                Some(Session::new(maybe_session_id.unwrap()))
            })
    }
}

