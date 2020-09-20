use serde_derive::Deserialize;

#[derive(Deserialize)]
pub(crate) struct RegisterForm {
    username: String,
    password: String,
}

impl RegisterForm {
    pub(crate) fn username(&self) -> &str {
        &self.username
    }

    pub(crate) fn password(&self) -> &str {
        &self.password
    }
}

#[derive(Deserialize)]
pub(crate) struct LoginForm {
    username: String,
    password: String,
}

impl LoginForm {
    pub(crate) fn username(&self) -> &str {
        &self.username
    }

    pub(crate) fn password(&self) -> &str {
        &self.password
    }
}
