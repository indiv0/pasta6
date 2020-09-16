use async_trait::async_trait;
use chrono::{DateTime, Utc};
use deadpool_postgres::Client;
use pasta6_core::{Error, Session};
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
