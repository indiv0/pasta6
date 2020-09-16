use async_trait::async_trait;
use super::models::RegisterForm;
use pasta6_core::{Error, Session};
pub(crate) use postgres::PostgresStore;
// TODO: call this `User` and the trait `UserTrait`
pub(crate) use postgres::User as UserStruct;

mod postgres;

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
