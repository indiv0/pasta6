use crate::{with_db, Error, Error::DbQueryError};
use async_trait::async_trait;
use deadpool_postgres::{Client, Pool};
use warp::{reject, Filter, Rejection};
use tokio_postgres::Row;
use std::convert::Infallible;

pub use session::Session;
pub use user::{BaseUser, User};

mod session;
mod user;

// We use the table `p6_user` because `user` is a reserved keyword in postgres.
const USER_TABLE: &str = "p6_user";
const USER_SELECT_FIELDS: &str = "id, username";
pub const SESSION_COOKIE_NAME: &str = "__Secure-session";

#[async_trait]
pub trait UserStore {
    type User: User;

    async fn get_user_by_session_id(
        db: &Client,
        session: &Session,
    ) -> Result<Option<Self::User>, Error>;
}

pub struct CoreUserStore;

#[async_trait]
impl UserStore for CoreUserStore {
    type User = BaseUser;

    // TODO: we really only need the username here, so why fetch the whole user?
    async fn get_user_by_session_id(
        db: &Client,
        session: &Session,
    ) -> Result<Option<Self::User>, Error> {
        let query = format!("SELECT {} FROM {} WHERE session = $1", USER_SELECT_FIELDS, USER_TABLE);
        let row = db
            .query_opt(query.as_str(), &[&session.session_id()])
            .await
            .map_err(DbQueryError)?;
        Ok(row.map(|r| row_to_user(&r)))
    }
}

pub fn optional_user<S>(
    pool: Pool,
) -> impl Filter<Extract = (Option<S::User>,), Error = Rejection> + Clone
    where S: UserStore,
{
    optional_session()
        .and(with_db(pool))
        .and_then(|maybe_session, db| async move {
            if let None = maybe_session {
                return Ok(None);
            }

            S::get_user_by_session_id(&db, &maybe_session.unwrap())
                .await
                .map_err(|e| reject::custom(e))
        })
}

// TODO: only load the session if it's present in the DB
pub fn optional_session(
) -> impl Filter<Extract = (Option<Session>,), Error = Infallible> + Clone {
    warp::filters::cookie::optional(SESSION_COOKIE_NAME).map(|maybe_cookie: Option<String>| {
        if let None = maybe_cookie {
            return None;
        }

        let maybe_session_id: Option<String> = serde_json::from_str(&maybe_cookie.unwrap())
            .map_err(|e| eprintln!("failed to deserialize session cookie: {:?}", e))
            .ok();
        if let None = maybe_session_id {
            return None;
        }

        Some(Session::new(maybe_session_id.unwrap()))
    })
}

// TODO: does this belong here or in models?
pub fn row_to_user(row: &Row) -> BaseUser {
    let id = row.get(0);
    //let created_at = row.get(1);
    let username = row.get(1);
    //let password = row.get(3);
    //let session = row.get(4);
    BaseUser::new(id, username)
}
