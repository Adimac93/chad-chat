use config::{Config, ConfigError};
use lettre::{transport::smtp::authentication::Credentials, Address};
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;
use std::net::SocketAddr;
use tracing::info;

#[derive(Deserialize, Clone)]
pub struct Settings {
    pub app: ApplicationSettings,
    pub postgres: PostgresSettings,
    pub redis: RedisSettings,
    pub smtp: SmtpSettings,
}

#[derive(Deserialize, Clone)]
pub struct SmtpSettings {
    username: Secret<String>,
    password: Secret<String>,
    pub relay: String,
    address: String,
}

impl SmtpSettings {
    pub fn get_credentials(&self) -> Credentials {
        Credentials::new(
            self.username.expose_secret().to_owned(),
            self.password.expose_secret().to_owned(),
        )
    }
    pub fn get_address(&self) -> Address {
        self.address.parse::<Address>().unwrap()
    }

    fn from_env() -> Self {
        let config = Config::builder()
            .add_source(config::Environment::with_prefix("SMTP").separator("_"))
            .build()
            .unwrap();
        config.try_deserialize().unwrap()
    }
}

#[derive(Deserialize, Clone)]
pub struct ApplicationSettings {
    pub host: String,
    pub port: u16,
    pub access_jwt_secret: Secret<String>,
    pub refresh_jwt_secret: Secret<String>,
    pub origin: String,
}

impl ApplicationSettings {
    pub fn get_addr(&self) -> SocketAddr {
        let addr = format!("{}:{}", self.host, self.port);
        addr.parse::<SocketAddr>()
            .expect(&format!("Failed to parse address: {addr} "))
    }

    pub fn from_env() -> Self {
        Self {
            host: "0.0.0.0".into(),
            port: get_env("PORT").parse::<u16>().expect("Invalid port number"),
            access_jwt_secret: get_secret_env("ACCESS_JWT_SECRET"),
            refresh_jwt_secret: get_secret_env("REFRESH_JWT_SECRET"),
            origin: get_env("WEBSITE_URL"),
        }
    }
}

#[derive(Deserialize, Clone)]
pub struct DatabaseFields {
    username: String,
    password: Secret<String>,
    port: u16,
    host: String,
    database_name: String,
}

impl DatabaseFields {
    fn compose(&self, db_name: String) -> String {
        format!(
            "{db_name}://{}:{}@{}:{}/{}",
            self.username,
            self.password.expose_secret(),
            self.host,
            self.port,
            self.database_name
        )
    }
}

pub trait ConnectionPrep {
    fn compose_database_url(&self) -> Option<String>;
    fn get_database_url(&self) -> Option<String>;
    fn env_database_url() -> Option<String>;
    fn get_connection_string(&self) -> String
        where
            Self: ToString,
    {
        let info = format!("url for {}", self.to_string());
        if let Some(url) = self.compose_database_url() {
            info!("Using composed {info}");
            url
        } else {
            if let Some(url) = self.get_database_url() {
                info!("Using field {info}");
                url
            } else {
                let url = Self::env_database_url().expect("No connection info provided");
                info!("Using env {info}");
                url
            }
        }
    }
}

#[derive(Deserialize, Clone)]
pub struct PostgresSettings {
    database_url: Option<String>,
    fields: Option<DatabaseFields>,
    is_migrating: Option<bool>,
}

impl PostgresSettings {
    pub fn is_migrating(&self) -> bool {
        self.is_migrating.unwrap_or(false)
    }
    fn from_env() -> Self {
        Self {
            database_url: Self::env_database_url(),
            fields: None,
            is_migrating: Some(true),
        }
    }
}

impl ToString for PostgresSettings {
    fn to_string(&self) -> String {
        String::from("postgresql")
    }
}

impl ConnectionPrep for PostgresSettings {
    fn compose_database_url(&self) -> Option<String> {
        Some(self.fields.clone()?.compose(self.to_string()))
    }
    fn get_database_url(&self) -> Option<String> {
        self.database_url.clone()
    }
    fn env_database_url() -> Option<String> {
        try_get_env("DATABASE_URL")
    }
}

#[derive(Deserialize, Clone)]
pub struct RedisSettings {
    database_url: Option<String>,
    fields: Option<DatabaseFields>,
}


impl RedisSettings {
    fn from_env() -> Self {
        Self {
            database_url: Self::env_database_url(),
            fields: None,
        }
    }
}

impl ToString for RedisSettings {
    fn to_string(&self) -> String {
        String::from("redis")
    }
}

impl ConnectionPrep for RedisSettings {
    fn compose_database_url(&self) -> Option<String> {
        Some(self.fields.clone()?.compose(self.to_string()))
    }
    fn get_database_url(&self) -> Option<String> {
        self.database_url.clone()
    }
    fn env_database_url() -> Option<String> {
        try_get_env("REDIS_URL")
    }
}

enum Environment {
    Local,
    Production,
}

impl TryFrom<String> for Environment {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "local" => Ok(Self::Local),
            "production" => Ok(Self::Production),
            other => Err(format!(
                "{other} is not supported environment. Use either `local` or `production`"
            )),
        }
    }
}

pub fn get_config() -> Result<Settings, ConfigError> {
    let base_path = std::env::current_dir().expect("Failed to determine the current directory");
    let config_dir = base_path.join("configuration");

    let environment: Environment = std::env::var("APP_ENVIRONMENT")
        .map_or(Environment::Local, |env| {
            env.try_into().expect("Failed to parse APP_ENVIRONMENT.")
        });

    match environment {
        Environment::Local => {
            let settings = Config::builder()
                .add_source(config::File::from(config_dir.join("settings.toml")))
                .add_source(
                    config::Environment::with_prefix("APP")
                        .prefix_separator("_")
                        .separator("__"),
                );
            return settings.build()?.try_deserialize();
        }

        Environment::Production => {
            let settings = Settings {
                app: ApplicationSettings::from_env(),
                postgres: PostgresSettings::from_env(),
                redis: RedisSettings::from_env(),
                smtp: SmtpSettings::from_env(),
            };
            return Ok(settings);
        }
    }
}

fn try_get_env(name: &str) -> Option<String> {
    std::env::var(name).ok()
}

fn try_get_secret_env(name: &str) -> Option<Secret<String>> {
    Some(Secret::from(try_get_env(name)?))
}

fn get_env(name: &str) -> String {
    std::env::var(name).expect(format!("Missing {name}").as_str())
}

fn get_secret_env(name: &str) -> Secret<String> {
    Secret::from(get_env(name))
}
