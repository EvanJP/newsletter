use std::net::TcpListener;

use newsletter::configuration::get_configuration;
use newsletter::email_client::EmailClient;
use newsletter::startup::run;
use newsletter::telemetry::get_subscriber;
use newsletter::telemetry::init_subscriber;
use reqwest::Url;
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Set up logging.
    let subscriber =
        get_subscriber("newsletter".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let config = get_configuration().expect("Failed to read configuration.");

    let connection_pool = PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy_with(config.database.with_db());
    let address =
        format!("{}:{}", config.application.host, config.application.port);
    let sender_email = config
        .email_client
        .sender()
        .expect("Invalid sender email address.");
    let email_client = EmailClient::new(
        Url::parse(config.email_client.base_url.as_str())
            .expect("Failed to pares URL"),
        sender_email,
        config.email_client.authorization_token,
    );
    let listener = TcpListener::bind(address)?;
    run(listener, connection_pool, email_client)?.await?;
    Ok(())
}
