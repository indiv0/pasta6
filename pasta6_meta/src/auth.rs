use async_trait::async_trait;
use chrono::{DateTime, Utc};
use pasta6_core::{AuthProvider, Token, User, UserStore};
use tokio_postgres::{GenericClient, Row};

use crate::hash::{verify, Hash};

macro_rules! user_table {
    () => {
        "\"user\""
    };
}

#[derive(Debug)]
pub(crate) struct MetaUser {
    // TODO: look into u32 for identifiers here and elsewhere
    id: i32,
    created_at: DateTime<Utc>,
    username: String,
    password: Hash,
}

impl MetaUser {
    pub(crate) fn new(
        id: i32,
        created_at: DateTime<Utc>,
        username: String,
        password: Hash,
    ) -> Self {
        Self {
            id,
            created_at,
            username,
            password,
        }
    }

    pub(crate) fn created_at(&self) -> &DateTime<Utc> {
        &self.created_at
    }
}

impl User for MetaUser {
    fn username(&self) -> &str {
        &self.username
    }
}

pub(crate) fn verify_password(user: &MetaUser, password: &str) -> bool {
    verify(&user.password, password)
}

pub(crate) struct MetaUserStore {}

impl MetaUserStore {
    pub(crate) async fn create_user<C>(
        client: &C,
        username: &str,
        password: &Hash,
    ) -> Result<MetaUser, tokio_postgres::Error>
    where
        C: GenericClient + Send + Sync,
    {
        const QUERY: &str = concat!(
            "INSERT INTO ",
            user_table!(),
            "(username, password) VALUES ($1, $2) RETURNING *"
        );
        let row = client.query_one(QUERY, &[&username, &password]).await?;
        Ok(row_to_user(&row))
    }
}

#[async_trait]
impl UserStore for MetaUserStore {
    type User = MetaUser;

    async fn get_user_by_username<C>(
        client: &C,
        username: &str,
    ) -> Result<Option<Self::User>, tokio_postgres::Error>
    where
        C: GenericClient + Send + Sync + 'static,
    {
        const QUERY: &str = concat!(
            "SELECT id, created_at, username, password FROM ",
            user_table!(),
            " WHERE username = $1"
        );
        Ok(match client.query_opt(QUERY, &[&username]).await? {
            Some(row) => Some(row_to_user(&row)),
            None => None,
        })
    }
}

fn row_to_user(row: &Row) -> MetaUser {
    let id = row.get(0);
    let created_at = row.get(1);
    let username = row.get(2);
    let password = row.get(3);
    MetaUser::new(id, created_at, username, password)
}

pub(crate) struct MetaAuthProvider {}

#[async_trait]
impl AuthProvider<MetaUserStore> for MetaAuthProvider {
    type User = MetaUser;

    async fn get_user<C>(
        client: &C,
        token: &Token,
    ) -> Result<Option<Self::User>, tokio_postgres::Error>
    where
        C: GenericClient + Send + Sync + 'static,
    {
        MetaUserStore::get_user_by_username(client, token.username()).await
    }
}
