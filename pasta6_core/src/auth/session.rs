use serde::Serialize;

#[derive(Serialize)]
pub struct Session(String);

impl Session {
    pub fn new(session_id: String) -> Self {
        Self(session_id)
    }

    pub fn session_id(&self) -> &str {
        &self.0
    }
}
