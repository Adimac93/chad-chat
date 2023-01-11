use config::{Config, ConfigError};
use lettre::transport::smtp::authentication::Credentials;
use secrecy::{ExposeSecret, Secret};
use serde::Deserialize;
use std::net::SocketAddr;
use tracing::info;

#[derive(Deserialize, Clone)]
pub struct Settings {
    pub app: ApplicationSettings,
    pub database: DatabaseSettings,
    pub smtp: SmtpSettings,
}

#[derive(Deserialize, Clone)]
pub struct SmtpSettings {
    username: Secret<String>,
    password: Secret<String>,
    pub relay: String,
}

impl SmtpSettings {
    pub fn get_credentials(self) -> Credentials {
        Credentials::new(
            self.username.expose_secret().to_owned(),
            self.password.expose_secret().to_owned(),
        )
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
}

#[derive(Deserialize, Clone)]
pub struct DatabaseSettings {
    database_url: Option<Secret<String>>,
    fields: Option<DatabaseFields>,
    is_migrating: Option<bool>,
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
    fn compose(&self) -> String {
        format!(
            "postgresql://{}:{}@{}:{}/{}",
            self.username,
            self.password.expose_secret(),
            self.host,
            self.port,
            self.database_name
        )
    }
}

impl DatabaseSettings {
    pub fn get_connection_string(&self) -> String {
        // database_url -> fields -> .env
        match &self.database_url {
            Some(url) => {
                info!("Database url using toml 'database_url'");
                url.expose_secret().to_string()
            }
            None => match &self.fields {
                Some(fields) => {
                    info!("Database url using toml 'fields'");
                    fields.compose()
                }
                None => {
                    info!("Database url using environment variable");
                    get_env("DATABASE_URL")
                }
            },
        }
    }

    pub fn is_migrating(&self) -> bool {
        self.is_migrating.unwrap_or(false)
    }

    fn production() -> Self {
        Self {
            database_url: None,
            fields: None,
            is_migrating: Some(true),
        }
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
                app: ApplicationSettings {
                    host: "0.0.0.0".into(),
                    port: get_env("PORT").parse::<u16>().expect("Invalid port number"),
                    access_jwt_secret: get_secret_env("ACCESS_JWT_SECRET"),
                    refresh_jwt_secret: get_secret_env("REFRESH_JWT_SECRET"),
                    origin: get_env("FRONTEND_URL"),
                },
                database: DatabaseSettings::production(),
                smtp: SmtpSettings {
                    username: get_secret_env("SMTP_USERNAME"),
                    password: get_secret_env("SMTP_PASSWORD"),
                    relay: get_env("SMTP_RELAY"),
                },
            };
            return Ok(settings);
        }
    }
}

fn get_env(name: &str) -> String {
    std::env::var(name).expect(format!("Missing {name}").as_str())
}
fn get_secret_env(name: &str) -> Secret<String> {
    Secret::from(get_env(name))
}
