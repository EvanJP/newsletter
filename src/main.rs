use std::net::TcpListener;

use newsletter::configuration::get_configuration;
use newsletter::startup::run;
use newsletter::telemetry::get_subscriber;
use newsletter::telemetry::init_subscriber;
use secrecy::ExposeSecret;
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Set up logging.
    let subscriber =
        get_subscriber("newsletter".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let config = get_configuration().expect("Failed to read configuration.");
    // let connection_pool =
    //     PgPool::connect(config.database.connection_string().expose_secret())
    //         .await
    //         .expect("Failed to connect to Postgres.");
    let connection_pool = PgPoolOptions::new()
        .acquire_timeout(std::time::Duration::from_secs(2))
        .connect_lazy(config.database.connection_string().expose_secret())
        .unwrap();
    // .connect_lazy_with(config.database.connection_string().expose_secret());
    let address =
        format!("{}:{}", config.application.host, config.application.port);
    let listener = TcpListener::bind(address)?;
    run(listener, connection_pool)?.await
}
