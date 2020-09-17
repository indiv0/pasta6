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
    _password: String,
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
            _password: password,
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
}
