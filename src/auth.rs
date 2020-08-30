pub use db::init_db;
pub use models::User;
pub use routes::{optional_user, routes};

mod models {
    use chrono::{DateTime, Utc};
    use serde_derive::Deserialize;

    #[derive(Deserialize)]
    #[serde(transparent)]
    pub struct Form<T>(T);

    #[derive(Deserialize)]
    pub struct RegisterForm {
        username: String,
        password: String,
    }

    impl RegisterForm {
        pub fn username(&self) -> &str {
            &self.username
        }

        pub fn password(&self) -> &str {
            &self.password
        }
    }

    // TODO: remove debug from here so we don't print the password accidentally
    #[derive(Debug)]
    pub struct User {
        // TODO: look into u32 for identifiers here and elsewhere
        id: i32,
        _created_at: DateTime<Utc>,
        username: String,
        _password: String,
        _session: Option<String>,
    }

    impl User {
        pub fn new(
            id: i32,
            created_at: DateTime<Utc>,
            username: String,
            password: String,
            session: Option<String>,
        ) -> Self {
            Self {
                id,
                _created_at: created_at,
                username,
                _password: password,
                _session: session,
            }
        }

        pub fn id(&self) -> &i32 {
            &self.id
        }

        pub fn username(&self) -> &str {
            &self.username
        }
    }
}

// TODO: make this private once main no longer calls get_user_by_session_id
mod db {
    use super::models::{RegisterForm, User};
    use crate::error::Error;
    use crate::session::Session;
    use deadpool_postgres::Client as DbClient;

    // We use the table `p6_user` because `user` is a reserved keyword in postgres.
    const TABLE: &str = "p6_user";
    const SELECT_FIELDS: &str = "id, created_at, username, password, session";

    pub async fn init_db(client: &DbClient) -> Result<(), tokio_postgres::Error> {
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

    pub async fn create_user(db: &DbClient, form: &RegisterForm) -> Result<User, Error> {
        // TODO: use a prepared statement.
        let query = format!(
            "INSERT INTO {} (username, password) VALUES ($1, $2) RETURNING *",
            TABLE
        );
        let row = db
            .query_one(query.as_str(), &[&form.username(), &form.password()])
            .await
            .map_err(Error::DbQueryError)?;
        Ok(row_to_user(&row))
    }

    pub async fn set_session(db: &DbClient, user: &User, session: &Session) -> Result<(), Error> {
        let query = format!("UPDATE {} SET session = $1 WHERE id = $2", TABLE);
        let row_count = db
            .execute(query.as_str(), &[&session.session_id(), user.id()])
            .await
            .map_err(Error::DbQueryError)?;
        // TODO: what about the case where we're updating a no-longer existent user?
        assert_eq!(row_count, 1);
        Ok(())
    }

    // TODO: we really only need the username here, so why fetch the whole user?
    pub async fn get_user_by_session_id(
        db: &DbClient,
        session: &Session,
    ) -> Result<Option<User>, Error> {
        let query = format!("SELECT {} FROM {} WHERE session = $1", SELECT_FIELDS, TABLE);
        let row = db
            .query_opt(query.as_str(), &[&session.session_id()])
            .await
            .map_err(Error::DbQueryError)?;
        Ok(row.map(|r| row_to_user(&r)))
    }

    // TODO: does this belong here or in models?
    fn row_to_user(row: &tokio_postgres::row::Row) -> User {
        let id = row.get(0);
        let created_at = row.get(1);
        let username = row.get(2);
        let password = row.get(3);
        let session = row.get(4);
        User::new(id, created_at, username, password, session)
    }
}

mod filter {
    use super::db;
    use super::models::{RegisterForm, User};
    use crate::filter::TemplateContext;
    use crate::session::{Session, SESSION_COOKIE_NAME};
    use askama_warp::Template;
    use deadpool_postgres::Client as DbClient;
    use rand::Rng;
    use warp::http::Uri;

    #[derive(Template)]
    #[template(path = "register.html")]
    struct RegisterTemplate {
        ctx: TemplateContext,
    }

    pub async fn get_register(
        current_user: Option<User>,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        Ok(RegisterTemplate {
            ctx: TemplateContext::new(current_user),
        })
    }

    pub async fn post_register(
        form: RegisterForm,
        db: DbClient,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        // TODO: perform proper validation to ensure these aren't empty values and enforce limits
        // on them (e.g. username length).
        // TODO: perform the validation in middleware.
        let user = db::create_user(&db, &form)
            .await
            .map_err(|e| warp::reject::custom(e))?;
        let redirect_uri = Uri::from_static("/");
        // TODO: generate the session ID in a cryptographically secure way.
        let session_id = rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(30)
            .collect();
        let session = Session::new(session_id);
        db::set_session(&db, &user, &session)
            .await
            .map_err(|e| warp::reject::custom(e))?;
        // TODO: should I be using serde_json to serialize the cookie or something like percent
        // encoding?
        let session_cookie = format!(
            "{}={}",
            SESSION_COOKIE_NAME,
            serde_json::to_string(&session).unwrap()
        );
        Ok(warp::redirect::redirect(redirect_uri)).map(|reply| {
            warp::reply::with_header(reply, warp::http::header::SET_COOKIE, session_cookie)
        })
    }
}

mod routes {
    use super::{db, filter, models};
    use crate::filter::with_db;
    use crate::routes::form_body;
    use crate::session;
    use deadpool_postgres::Pool as DbPool;
    use warp::Filter;

    pub fn optional_user(
        pool: DbPool,
    ) -> impl Filter<Extract = (Option<models::User>,), Error = warp::Rejection> + Clone {
        session::optional_session()
            .and(with_db(pool))
            .and_then(|maybe_session, db| async move {
                if let None = maybe_session {
                    return Ok(None);
                }

                db::get_user_by_session_id(&db, &maybe_session.unwrap())
                    .await
                    .map_err(|e| warp::reject::custom(e))
            })
    }

    /// GET /register
    fn get_register(
        pool: DbPool,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path("register")
            .and(warp::get())
            .and(optional_user(pool))
            .and_then(filter::get_register)
    }

    /// POST /register
    fn post_register(
        pool: DbPool,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        warp::path("register")
            .and(warp::post())
            // TODO: if we submit a malformed form (e.g. no `input` with `name="username"` then on the console we see:
            //
            //     body deserialize error: BodyDeserializeError { cause: Error { err: "missing field `username`" } }
            //
            //  The JSON response is just `{"message": "Invalid body"}`. We should probably take
            //  users to a 4xx page or display a proper error on the website in this scenario.
            .and(form_body())
            .and(with_db(pool))
            .and_then(filter::post_register)
    }

    pub fn routes(
        pool: DbPool,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        get_register(pool.clone()).or(post_register(pool))
    }
}
