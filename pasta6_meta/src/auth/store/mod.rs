use async_trait::async_trait;
use super::models::RegisterForm;
use pasta6_core::{Error, Session, User};
use chrono::{Utc, DateTime};
pub(crate) use postgres::PostgresStore;

mod postgres;

pub(crate) struct MetaUser {
    // TODO: look into u32 for identifiers here and elsewhere
    id: i32,
    created_at: DateTime<Utc>,
    username: String,
    password: String,
    _session: Option<String>,
}

impl MetaUser {
    pub(crate) fn new(
        id: i32,
        created_at: DateTime<Utc>,
        username: String,
        password: String,
        session: Option<String>,
    ) -> Self {
        Self {
            id,
            created_at,
            username,
            password,
            _session: session,
        }
    }

    pub(crate) fn created_at(&self) -> &DateTime<Utc> {
        &self.created_at
    }
}

impl User for MetaUser {
    fn id(&self) -> i32 {
        self.id
    }

    fn username(&self) -> &str {
        &self.username
    }
}

#[async_trait]
pub(crate) trait UserStore {
    async fn create_user(&self, form: &RegisterForm) -> Result<MetaUser, Error>;

    async fn set_session<U>(&self, user: &U, session: &Session) -> Result<(), Error>
        where U: User + Sync;

    async fn unset_session(&self, session: &Session) -> Result<(), Error>;

    async fn get_user_by_username(&self, username: &str) -> Result<Option<MetaUser>, Error>;
}

pub(crate) fn verify_password(user: &MetaUser, password: &str) -> bool {
    user.password == password
}