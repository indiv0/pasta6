use crate::auth::hash::Hash;

use super::MetaUser;
use super::UserStore;
use async_trait::async_trait;
use deadpool_postgres::Client;
use pasta6_core::{Error, Session, User};
use tokio_postgres::{GenericClient, Row};

const TABLE: &str = "p6_user";
const SELECT_FIELDS: &str = "id, created_at, username, password, session";

pub(crate) struct PostgresStore<'a, C>
where
    C: GenericClient,
{
    db: &'a C,
}

impl<'a, C> PostgresStore<'a, C>
where
    C: GenericClient,
{
    pub(crate) fn new(db: &'a C) -> Self {
        Self { db }
    }
}

#[async_trait]
impl<C> UserStore for PostgresStore<'_, C>
where
    C: GenericClient + Send + Sync,
{
    async fn create_user(&self, username: &str, password: &Hash) -> Result<MetaUser, Error> {
        // TODO: use a prepared statement.
        let query = format!(
            "INSERT INTO {} (username, password) VALUES ($1, $2) RETURNING *",
            TABLE
        );
        let row = self
            .db
            .query_one(query.as_str(), &[&username, &password])
            .await
            .map_err(Error::DbQueryError)?;
        Ok(row_to_user(&row))
    }

    async fn set_session<U>(&self, user: &U, session: &Session) -> Result<(), Error>
    where
        U: User + Sync,
    {
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

    async fn unset_session(&self, session: &Session) -> Result<(), Error> {
        let query = format!("UPDATE {} SET session = NULL WHERE session = $1", TABLE);
        let row_count = self
            .db
            .execute(query.as_str(), &[&session.session_id()])
            .await
            .map_err(Error::DbQueryError)?;
        // TODO: are we OK with this returning 0 if we're un-setting an already unset session?
        // TODO: this can potentially be abused to unset multiple users' sessions at once, if the UNIQUE constraint on sessions is removed.
        assert_eq!(row_count, 1);
        Ok(())
    }

    async fn get_user_by_username(&self, username: &str) -> Result<Option<MetaUser>, Error> {
        let query = format!(
            "SELECT {} FROM {} WHERE username = $1",
            SELECT_FIELDS, TABLE
        );
        let row = self
            .db
            .query_opt(query.as_str(), &[&username])
            .await
            .map_err(Error::DbQueryError)?;
        Ok(row.as_ref().map(row_to_user))
    }
}

#[async_trait]
impl<'a, C> pasta6_core::UserStore for PostgresStore<'a, C>
where
    C: GenericClient + Send + Sync + 'static,
{
    type User = MetaUser;

    // TODO: we really only need the username here, so why fetch the whole user?
    async fn get_user_by_session_id(
        db: &Client,
        session: &Session,
    ) -> Result<Option<MetaUser>, Error> {
        let query = format!("SELECT {} FROM {} WHERE session = $1", SELECT_FIELDS, TABLE);
        let row = db
            .query_opt(query.as_str(), &[&session.session_id()])
            .await
            .map_err(Error::DbQueryError)?;
        Ok(row.as_ref().map(row_to_user))
    }
}

// TODO: does this belong here or in models?
fn row_to_user(row: &Row) -> MetaUser {
    let id = row.get(0);
    let created_at = row.get(1);
    let username = row.get(2);
    let password = row.get(3);
    let session = row.get(4);
    MetaUser::new(id, created_at, username, password, session)
}
