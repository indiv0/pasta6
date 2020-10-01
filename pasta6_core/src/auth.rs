use crate::config::SecretKey;
use async_trait::async_trait;
use bronco::EncodeError;
use serde::{Deserialize, Serialize};
use tokio_postgres::{GenericClient, Row};

// We quote the table name `user` because `user` is a reserved keyword in postgres.
macro_rules! user_table {
    () => {
        "\"user\""
    };
}

#[derive(Debug)]
pub struct CoreUser {
    username: String,
}

impl CoreUser {
    pub fn new(username: String) -> Self {
        Self { username }
    }
}

pub trait User {
    fn username(&self) -> &str;
}

impl User for CoreUser {
    fn username(&self) -> &str {
        &self.username
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Token {
    username: String,
}

impl Token {
    pub fn new(username: String) -> Self {
        Self { username }
    }

    pub fn username(&self) -> &str {
        &self.username
    }

    pub fn encode(user: &impl User, secret_key: &SecretKey) -> Result<String, serde_json::Error> {
        let token = Self::new(user.username().to_owned());
        let json = serde_json::to_string(&token)?;
        match bronco::encode(&json, secret_key.as_bytes()) {
            Ok(encoded) => Ok(encoded),
            Err(EncodeError::WrongKeyLength) => unreachable!(),
        }
    }
}

#[async_trait]
pub trait UserStore {
    type User: User;

    async fn get_user_by_username<C>(
        client: &C,
        username: &str,
    ) -> Result<Option<Self::User>, tokio_postgres::Error>
    where
        C: GenericClient + Send + Sync + 'static;
}

pub struct CoreUserStore;

impl CoreUserStore {
    async fn insert_user<C>(client: &C, username: &str) -> Result<CoreUser, tokio_postgres::Error>
    where
        C: GenericClient + Send + Sync + 'static,
    {
        const QUERY: &str = concat!(
            "INSERT INTO ",
            user_table!(),
            " (username) VALUES ($1) RETURNING *"
        );
        let row = client.query_one(QUERY, &[&username]).await?;
        Ok(CoreUser::new(row.get(2)))
    }
}

#[async_trait]
impl UserStore for CoreUserStore {
    type User = CoreUser;

    async fn get_user_by_username<C>(
        client: &C,
        username: &str,
    ) -> Result<Option<Self::User>, tokio_postgres::Error>
    where
        C: GenericClient + Send + Sync + 'static,
    {
        const QUERY: &str = concat!(
            "SELECT username FROM ",
            user_table!(),
            " WHERE username = $1"
        );
        Ok(client
            .query_opt(QUERY, &[&username])
            .await?
            .as_ref()
            .map(row_to_user))
    }
}

#[async_trait]
pub trait AuthProvider<S>
where
    S: UserStore,
{
    type User: User;

    async fn get_user<C>(
        client: &C,
        token: &Token,
    ) -> Result<Option<Self::User>, tokio_postgres::Error>
    where
        C: GenericClient + Send + Sync + 'static;
}

pub struct CoreAuthProvider {}

#[async_trait]
impl AuthProvider<CoreUserStore> for CoreAuthProvider {
    type User = CoreUser;

    async fn get_user<C>(
        client: &C,
        token: &Token,
    ) -> Result<Option<Self::User>, tokio_postgres::Error>
    where
        C: GenericClient + Send + Sync + 'static,
    {
        Ok(Some(
            match CoreUserStore::get_user_by_username(client, token.username()).await? {
                Some(user) => user,
                None => CoreUserStore::insert_user(client, token.username()).await?,
            },
        ))
    }
}

fn row_to_user(row: &Row) -> CoreUser {
    let username = row.get(0);
    CoreUser::new(username)
}
