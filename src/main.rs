use newsletter::configuration::get_configuration;
use newsletter::startup::Application;
use newsletter::telemetry::get_subscriber;
use newsletter::telemetry::init_subscriber;

/// Starts up the web-app.
#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    let subscriber =
        get_subscriber("newsletter".into(), "info".into(), std::io::stdout);
    init_subscriber(subscriber);

    let config = get_configuration().expect("Failed to read configuration.");
    let application = Application::build(config).await?;
    application.run_until_stopped().await?;
    Ok(())
}
