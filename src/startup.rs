//! For actually running the application.
use std::net::TcpListener;

use actix_web::dev::Server;
use actix_web::web;
use actix_web::App;
use actix_web::HttpServer;
use reqwest::Url;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tracing_actix_web::TracingLogger;

use crate::configuration::DatabaseSettings;
use crate::configuration::Settings;
use crate::email_client::EmailClient;
use crate::routes::confirm;
use crate::routes::health_check;
use crate::routes::publish_newsletter;
use crate::routes::subscribe;

/// Initializes a [`PgPool`] for multiple DB connections.
pub fn get_connection_pool(config: &DatabaseSettings) -> PgPool {
    PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(config.with_db())
}

/// Wrapper type for retrieving URL. Context-retrieval in actix-web is type
/// based, avoiding raw `String`s is advisable.
pub struct ApplicationBaseUrl(pub String);

/// The main app function.
///
/// Starts up an `HttpServer` with routing and necessary app data.
///
/// **Middleware**:
/// * [`TracingLogger`] - Middleware for capturing spans.
///
/// **Routes**:
/// * [`/health_check`](crate::routes::health_check)
/// * [`/subscriptions`](crate::routes::subscribe)
/// * [`/subscriptions/confirm`](crate::routes::confirm)
///
/// # Arguments
///
/// * `listener` - A TcpListener for the app.
/// * `db_pool ` - The Postgres DB connection interface.
/// * `email_client` - Our [`reqwest::Client`] wrapper for sending emails.
/// * `base_url` - The API's base url.
pub fn run(
    listener: TcpListener,
    db_pool: PgPool,
    email_client: EmailClient,
    base_url: String,
) -> Result<Server, std::io::Error> {
    // Arc returned.
    let connection = web::Data::new(db_pool);
    let email_client = web::Data::new(email_client);
    let base_url = web::Data::new(ApplicationBaseUrl(base_url));
    // Need to move connection in since the closure will outlive the connection
    // lifetime.
    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .route("/health_check", web::get().to(health_check))
            .route("/subscriptions", web::post().to(subscribe))
            .route("/subscriptions/confirm", web::get().to(confirm))
            .route("/newsletters", web::post().to(publish_newsletter))
            .app_data(connection.clone())
            .app_data(email_client.clone())
            .app_data(base_url.clone())
    })
    .listen(listener)?
    .run();

    Ok(server)
}

/// The Application builder.
pub struct Application {
    port: u16,
    server: Server,
}

impl Application {
    /// Build the application from the config.
    ///
    /// Establishes:
    /// * The address.
    /// * The sender email.
    /// * The email client.
    /// * The TcpListener.
    pub async fn build(config: Settings) -> Result<Self, std::io::Error> {
        let connection_pool = get_connection_pool(&config.database);

        let address =
            format!("{}:{}", config.application.host, config.application.port);
        let sender_email = config
            .email_client
            .sender()
            .expect("Invalid sender email address.");
        let timeout = config.email_client.timeout();
        let email_client = EmailClient::new(
            Url::parse(config.email_client.base_url.as_str())
                .expect("Failed to pares URL"),
            sender_email,
            config.email_client.authorization_token,
            timeout,
        );
        let listener = TcpListener::bind(address)?;
        let port = listener.local_addr().unwrap().port();

        let server = run(
            listener,
            connection_pool,
            email_client,
            config.application.base_url,
        )?;

        Ok(Self { port, server })
    }

    /// Returns the app's port.
    pub fn port(&self) -> u16 {
        self.port
    }

    // Start the server.
    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}
