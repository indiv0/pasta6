pub trait User {
    fn id(&self) -> i32;
    fn username(&self) -> &str;
}

// TODO: remove debug from here so we don't print the password accidentally
#[derive(Debug)]
pub struct BaseUser {
    // TODO: look into u32 for identifiers here and elsewhere
    id: i32,
    //_created_at: DateTime<Utc>,
    username: String,
    //_password: String,
    //_session: Option<String>,
}

impl BaseUser {
    pub fn new(
        id: i32,
        //created_at: DateTime<Utc>,
        username: String,
        //password: String,
        //session: Option<String>,
    ) -> Self {
        Self {
            id,
            //_created_at: created_at,
            username,
            //_password: password,
            //_session: session,
        }
    }
}

impl User for BaseUser {
    fn id(&self) -> i32 {
        self.id
    }

    fn username(&self) -> &str {
        &self.username
    }
}
