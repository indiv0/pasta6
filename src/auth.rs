pub(crate) use db::init_db;
pub(crate) use models::User;
pub(crate) use routes::{optional_user, routes};

mod store {
    use async_trait::async_trait;
    use crate::error::Error;
    use crate::session::Session;
    use super::models::RegisterForm;
    pub(crate) use postgres::PostgresStore;
    // TODO: call this `User` and the trait `UserTrait`
    pub(crate) use postgres::User as UserStruct;

    pub(crate) trait User {
        fn id(&self) -> i32;
    }

    #[async_trait]
    pub(crate) trait UserStore {
        type User: User;

        async fn create_user(&self, form: &RegisterForm) -> Result<Self::User, Error>;

        async fn set_session(&self, user: &Self::User, session: &Session) -> Result<(), Error>;

        async fn get_user_by_session(&self, session: &Session) -> Result<Option<Self::User>, Error>;
    }

    mod postgres {
        use async_trait::async_trait;
        use chrono::{DateTime, Utc};
        use crate::error::Error;
        use crate::session::Session;
        use deadpool_postgres::Client;
        use super::UserStore;
        use super::User as UserTrait;
        use super::super::models::RegisterForm;
        use tokio_postgres::Row;

        const TABLE: &str = "user";
        const SELECT_FIELDS: &str = "id, created_at, username, password, session";

        // TODO: this belongs in the above module, but then we'd have a naming conflict
        pub(crate) struct User {
            // TODO: look into u32 for identifiers here and elsewhere
            id: i32,
            _created_at: DateTime<Utc>,
            _username: String,
            _password: String,
            _session: Option<String>,
        }

        impl UserTrait for User {
            fn id(&self) -> i32 {
                self.id
            }
        }

        pub(crate) struct PostgresStore<'a> {
            db: &'a Client,
        }

        impl<'a> PostgresStore<'a> {
            pub(crate) fn new(db: &'a Client) -> Self {
                Self { db }
            }
        }

        #[async_trait]
        impl UserStore for PostgresStore<'_> {
            type User = User;

            async fn create_user(&self, form: &RegisterForm) -> Result<Self::User, Error> {
                // TODO: use a prepared statement.
                let query = format!(
                    "INSERT INTO {} (username, password) VALUES ($1, $2) RETURNING *",
                    TABLE
                );
                let row = self
                    .db
                    .query_one(query.as_str(), &[&form.username(), &form.password()])
                    .await
                    .map_err(Error::DbQueryError)?;
                Ok(FromPostgresRow::from_postgres_row(&row))
            }

            async fn set_session(&self, user: &Self::User, session: &Session) -> Result<(), Error> {
                let query = format!("UPDATE {} SET session = $1 WHERE id = $2", TABLE);
                let row_count = self
                    .db
                    .execute(query.as_str(), &[&session.session_id(), &user.id()])
                    .await
                    .map_err(Error::DbQueryError)?;
                // TODO: what about the case where we're updating a no-longer existent user?
                assert_eq!(row_count, 1);
                Ok(())
            }

            // TODO: we really only need the username here, so why fetch the whole user?
            async fn get_user_by_session(&self, session: &Session) -> Result<Option<Self::User>, Error> {
                let query = format!("SELECT {} FROM {} WHERE session = $1", SELECT_FIELDS, TABLE);
                let row = self
                    .db
                    .query_opt(query.as_str(), &[&session.session_id()])
                    .await
                    .map_err(Error::DbQueryError)?;
                Ok(row.as_ref().map(FromPostgresRow::from_postgres_row))
            }
        }

        trait FromPostgresRow: Sized {
            fn from_postgres_row(r: &Row) -> Self;
        }

        impl FromPostgresRow for User {
            fn from_postgres_row(r: &Row) -> Self {
                Self {
                    id: r.get(0),
                    _created_at: r.get(1),
                    _username: r.get(2),
                    _password: r.get(3),
                    _session: r.get(4),
                }
            }
        }
    }
}

mod models {
    use chrono::{DateTime, Utc};
    use serde_derive::Deserialize;

    #[derive(Deserialize)]
    #[serde(transparent)]
    pub(crate) struct Form<T>(T);

    #[derive(Deserialize)]
    pub(crate) struct RegisterForm {
        username: String,
        password: String,
    }

    impl RegisterForm {
        pub(crate) fn username(&self) -> &str {
            &self.username
        }

        pub(crate) fn password(&self) -> &str {
            &self.password
        }
    }

    // TODO: remove debug from here so we don't print the password accidentally
    #[derive(Debug)]
    pub(crate) struct User {
        // TODO: look into u32 for identifiers here and elsewhere
        id: i32,
        _created_at: DateTime<Utc>,
        username: String,
        _password: String,
        _session: Option<String>,
    }

    impl User {
        pub(crate) fn new(
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

        pub(crate) fn username(&self) -> &str {
            &self.username
        }
    }
}

// TODO: make this private once main no longer calls get_user_by_session_id
mod db {
    use super::store::{UserStore, PostgresStore};
    use super::models::{RegisterForm, User};
    // TODO: remove this alias
    use super::store::UserStruct;
    use crate::error::Error;
    use crate::session::Session;
    use deadpool_postgres::Client as DbClient;

    // We use the table `p6_user` because `user` is a reserved keyword in postgres.
    const TABLE: &str = "p6_user";
    const SELECT_FIELDS: &str = "id, created_at, username, password, session";

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

    // TODO: we really only need the username here, so why fetch the whole user?
    pub(crate) async fn get_user_by_session_id(
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

    pub(crate) async fn get_register(
        current_user: Option<User>,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        Ok(RegisterTemplate {
            ctx: TemplateContext::new(current_user),
        })
    }

    pub(crate) async fn post_register(
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

    pub(crate) fn optional_user(
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

    pub(crate) fn routes(
        pool: DbPool,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        get_register(pool.clone()).or(post_register(pool))
    }
}
