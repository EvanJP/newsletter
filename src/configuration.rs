use config::Config;
use config::ConfigError;
use config::File;
use config::FileFormat;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Settings {
    pub application_port: u16,
    pub database: DatabaseSettings,
}

#[derive(Deserialize)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: String,
    pub port: u16,
    pub host: String,
    pub database_name: String,
}

impl DatabaseSettings {
    pub fn connection_string(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username,
            self.password,
            self.host,
            self.port,
            self.database_name
        )
    }

    pub fn connection_string_without_db(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}",
            self.username, self.password, self.host, self.port
        )
    }
}

pub fn get_configuration() -> Result<Settings, ConfigError> {
    // Initialize our configuration reader
    let settings = Config::builder()
        // Add configuration values from yaml file.
        .add_source(File::new("configuration.yaml", FileFormat::Yaml))
        .build()?;
    // Try to convert it into settings type.
    settings.try_deserialize::<Settings>()
}
