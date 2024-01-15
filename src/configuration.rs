//! App's configuration crate.
use config::Config;
use config::ConfigError;
use config::File;
use secrecy::ExposeSecret;
use secrecy::Secret;
use serde::Deserialize;
use serde_aux::field_attributes::deserialize_number_from_string;
use sqlx::postgres::PgConnectOptions;
use sqlx::postgres::PgSslMode;
use sqlx::ConnectOptions;

use crate::domain::SubscriberEmail;

/// The app's overall settings holder.
#[derive(Deserialize, Clone)]
pub struct Settings {
    pub application: ApplicationSettings,
    pub database: DatabaseSettings,
    pub email_client: EmailClientSettings,
}

/// Base app settings.
///
/// * `port` - The port number.
/// * `host` - The host string.
/// * `base_url` - The url of this API to send requests to.
/// * `hmac_secret` - The [HMAC](https://en.wikipedia.org/wiki/HMAC) secret to
///   be appeneded to query params.
#[derive(Deserialize, Clone)]
pub struct ApplicationSettings {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
    pub base_url: String,
    pub hmac_secret: Secret<String>,
}

/// The Postgres database settings for the app.
///
/// * `username` - The DB username.
/// * `password` - The DB password, wrapped in a [`secrecy::Secret`].
/// * `port` - The port.
/// * `host` - The host string.
/// * `database_name` - The database name.
/// * `require_ssl` - Whether to require SSL or not. Note, because of [fly.io](https://fly.io)'s
///   private network, [SSL between app and DB is unnecessary and unsupported.](https://community.fly.io/t/seeking-advice-about-securing-postgres-on-fly/7861/2)
#[derive(Deserialize, Clone, Debug)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: Secret<String>,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
    pub database_name: String,
    pub require_ssl: bool,
}

impl DatabaseSettings {
    /// Initialize the [`PgConnectOptions`] *without* a db name.
    ///
    /// Necessary for usage with test DBs.
    pub fn without_db(&self) -> PgConnectOptions {
        let ssl_mode = if self.require_ssl {
            PgSslMode::Require
        } else {
            // Fallback to unencrypted if failed.
            PgSslMode::Disable
        };
        PgConnectOptions::new()
            .host(&self.host)
            .username(&self.username)
            .password(self.password.expose_secret())
            .port(self.port)
            .ssl_mode(ssl_mode)
    }

    /// Initialize the [`PgConnectOptions`] *with* a db name.
    ///
    /// Also includes log statements at a Trace level.
    pub fn with_db(&self) -> PgConnectOptions {
        self.without_db()
            .database(&self.database_name)
            .log_statements(tracing_log::log::LevelFilter::Trace)
    }
}

/// Settings for configuring the [`crate::email_client::EmailClient`].
///
/// * `base_url` - Holds the app's API url.
/// * `sender_email` - A string holding the email to send from.
/// * `authorization_token` - A [`secrecy::Secret`] String that holds our
///   sendgrid API key.
/// * `timeout_milliseconds` - The timeout for the client in milliseconds.
#[derive(Deserialize, Clone)]
pub struct EmailClientSettings {
    pub base_url: String,
    pub sender_email: String,
    pub authorization_token: Secret<String>,
    pub timeout_milliseconds: u64,
}

impl EmailClientSettings {
    /// Parse the email string to [`SubscriberEmail`].
    pub fn sender(&self) -> Result<SubscriberEmail, String> {
        SubscriberEmail::parse(self.sender_email.clone())
    }

    /// Parse the timeout to [`std::time::Duration`]
    pub fn timeout(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.timeout_milliseconds)
    }
}

/// Holding the app's env state.
pub enum Environment {
    /// As implied.
    Local,
    /// As implied.
    Production,
}

impl Environment {
    /// Maps `Local` to "local" and `Production` to "production".
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Local => "local",
            Environment::Production => "production",
        }
    }
}

impl TryFrom<String> for Environment {
    type Error = String;

    /// Maps "local" to `Local` and "production" to `Production`.
    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.to_lowercase().as_str() {
            "local" => Ok(Environment::Local),
            "production" => Ok(Environment::Production),
            other => Err(format!(
                "{} is not a supported environment. Use either `local` or \
                 `production`.",
                other
            )),
        }
    }
}

/// Returns the app's config.
///
/// Attempts to read from the `/configuration` folder. Will base it on the given
/// `APP_ENVIRONMENT` env variable. Parses from `.yaml` files.
pub fn get_configuration() -> Result<Settings, ConfigError> {
    // Initialize our configuration reader
    let base_path = std::env::current_dir()
        .expect("Failed to determine current directory.");
    let configuration_directory = base_path.join("configuration");

    // Detect running environment. Default to `local` if unspecified.
    let environment: Environment = std::env::var("APP_ENVIRONMENT")
        .unwrap_or_else(|_| "local".into())
        .try_into()
        .expect("Failed to parse APP_ENVIRONMENT.");
    let environment_filename = format!("{}.yaml", environment.as_str());
    let settings = Config::builder()
        .add_source(File::from(configuration_directory.join("base.yaml")))
        .add_source(File::from(
            configuration_directory.join(environment_filename),
        ))
        .add_source(
            config::Environment::with_prefix("APP")
                .prefix_separator("_")
                .separator("__"),
        )
        .build()?;

    settings.try_deserialize::<Settings>()
}
