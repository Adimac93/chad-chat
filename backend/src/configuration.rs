use config::{Config, ConfigError};
use secrecy::Secret;
use serde::Deserialize;
use std::net::SocketAddr;

#[derive(Deserialize, Clone)]
pub struct Settings {
    pub app: ApplicationSettings,
}

#[derive(Deserialize, Clone)]
pub struct ApplicationSettings {
    pub database_url: Secret<String>,
    pub host: String,
    pub port: u16,
    pub jwt_key: Secret<String>,
    pub refresh_jwt_key: Secret<String>,
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
    pub username: String,
    pub password: Secret<String>,
    pub port: u16,
    pub host: String,
    pub database_name: String,
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
                    jwt_key: Secret::from(get_env("JWT_SECRET")),
                    refresh_jwt_key: Secret::from(get_env("REFRESH_JWT_SECRET")),
                    database_url: Secret::from(get_env("DATABASE_URL")),
                    origin: get_env("FRONTEND_URL"),
                },
            };
            return Ok(settings);
        }
    }
}

fn get_env(name: &str) -> String {
    std::env::var(name).expect(format!("Missing {name}").as_str())
}
