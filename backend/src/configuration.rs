use config::{Config, ConfigError, File, FileFormat};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Setting {
    pub database: DatabaseSettings,
    pub addr: AddresSettings,
    pub jwt: JWTSettings,
}

#[derive(Deserialize)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: String,
    pub port: u16,
    pub host: String,
    pub database_name: String,
}

#[derive(Deserialize)]
pub struct AddresSettings {
    pub ip: [u8; 4],
    pub port: u16,
}

#[derive(Deserialize)]
pub struct JWTSettings {
    pub secret: String,
}

impl DatabaseSettings {
    pub fn connection_string(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username, self.password, self.host, self.port, self.database_name
        )
    }
}

pub fn get_config() -> Result<Setting, ConfigError> {
    let settings = Config::builder().add_source(File::new("config/settings", FileFormat::Toml));

    settings.build()?.try_deserialize()
}
