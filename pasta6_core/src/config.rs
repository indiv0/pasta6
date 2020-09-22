use std::{env, fs};
use toml::Value;
use tracing::trace;
use ConfigError::*;

#[derive(Debug)]
pub enum ConfigError {
    NotFound(String),
    WrongType(String),
}

pub struct Config {
    inner: Value,
}

impl Config {
    pub fn load() -> Self {
        let config_path = env::current_dir()
            .unwrap()
            .join(env::var("PASTA6_CONFIG").unwrap_or("config.toml".to_owned()));
        trace!("Loading config file: {}", config_path.to_str().unwrap());
        Config {
            inner: fs::read_to_string(config_path)
                .unwrap()
                .parse::<Value>()
                .unwrap(),
        }
    }

    pub fn get<'a, 'b>(&'a self, key: &'b str) -> Result<&str, ConfigError> {
        trace!("Reading config key: {}", key);
        self._get_nested(key)?
            .as_str()
            .ok_or_else(|| WrongType(key.to_owned()))
    }

    pub fn get_network<'a, 'b>(
        &'a self,
    ) -> Result<impl Iterator<Item = &'a String> + 'a, ConfigError> {
        Ok(self
            ._get("services")?
            .as_table()
            .ok_or_else(|| WrongType("services".to_owned()))?
            .keys())
    }

    pub fn get_service_domain<'a, 'b>(&'a self, service: &str) -> Result<&str, ConfigError> {
        self.get(&format!("services.{}.domain", service).to_owned())
    }

    fn _get_nested(&self, key: &str) -> Result<&Value, ConfigError> {
        let mut keys = key.split(".");
        let mut path = keys.next().unwrap().to_owned();
        let mut value = self._get(&path)?;
        for key in keys {
            path = format!("{}.{}", path, key);
            value = value.get(key).ok_or_else(|| NotFound(path.to_owned()))?;
        }
        Ok(value)
    }

    fn _get<'a, 'b>(&'a self, key: &'b str) -> Result<&Value, ConfigError> {
        self.inner.get(key).ok_or_else(|| NotFound(key.to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config() {
        let config_toml = toml::toml! {
            [pasta6]
            site_name="pasta6"
            domain="p6.rs"

            [services.meta]
            name="meta.p6.rs"
            domain="https://meta.p6.rs"

            [services.meta.database]
            host="localhost"
            user="pasta6"
            password="pasta6"
            dbname="meta.p6.rs"

            [services.home]
            name="home.p6.rs"
            domain="https://p6.rs"

            [services.home.database]
            host="localhost"
            user="pasta6"
            password="pasta6"
            dbname="home.p6.rs"

            [services.paste]
            name="paste.p6.rs"
            domain="https://paste.p6.rs"

            [services.paste.database]
            host="localhost"
            user="pasta6"
            password="pasta6"
            dbname="paste.p6.rs"
        };

        let cfg = Config { inner: config_toml };

        assert_matches!(cfg.get("foo"), Err(NotFound(k)) if k == "foo");
        assert_matches!(cfg.get("services"), Err(WrongType(k)) if k == "services");
        assert_eq!(
            cfg.get_network().unwrap().collect::<Vec<_>>(),
            vec!["home", "meta", "paste"]
        );

        assert_eq!(cfg.get_service_domain("home").unwrap(), "https://p6.rs");
        assert_eq!(
            cfg.get_service_domain("meta").unwrap(),
            "https://meta.p6.rs"
        );
        assert_eq!(
            cfg.get_service_domain("paste").unwrap(),
            "https://paste.p6.rs"
        );

        assert_eq!(
            cfg.get("services.home.database.dbname").unwrap(),
            "home.p6.rs"
        );
        assert_matches!(cfg.get("services.home.database.foo"), Err(NotFound(k)) if k == "services.home.database.foo");
        assert_matches!(cfg.get("services.home.database"), Err(WrongType(k)) if k == "services.home.database");
    }
}
