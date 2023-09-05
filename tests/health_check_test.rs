use std::net::TcpListener;

use newsletter::configuration::get_configuration;
use newsletter::configuration::DatabaseSettings;
use newsletter::startup::run;
use newsletter::telemetry::get_subscriber;
use newsletter::telemetry::init_subscriber;
use once_cell::sync::Lazy;
use secrecy::ExposeSecret;
use sqlx::Connection;
use sqlx::Executor;
use sqlx::PgConnection;
use sqlx::PgPool;
use uuid::Uuid;

static TRACING: Lazy<()> = Lazy::new(|| {
    let default_filter_level = "info".to_string();
    let subscriber_name = "test".to_string();
    if std::env::var("TEST_LOG").is_ok() {
        let subscriber = get_subscriber(
            subscriber_name,
            default_filter_level,
            std::io::stdout,
        );
        init_subscriber(subscriber);
    } else {
        let subscriber = get_subscriber(
            subscriber_name,
            default_filter_level,
            std::io::sink,
        );
        init_subscriber(subscriber);
    };
});

pub struct TestApp {
    pub address: String,
    pub db_pool: PgPool,
}

pub async fn configure_database(config: &DatabaseSettings) -> PgPool {
    let mut connection = PgConnection::connect(
        &config.connection_string_without_db().expose_secret(),
    )
    .await
    .expect("Failed to connect to Postgres");
    connection
        .execute(
            format!(r#"CREATE DATABASE "{}";"#, config.database_name).as_str(),
        )
        .await
        .expect("Failed to create database.");

    let db_pool = PgPool::connect(&config.connection_string().expose_secret())
        .await
        .expect("Failed to connect to Postgres.");
    sqlx::migrate!("./migrations")
        .run(&db_pool)
        .await
        .expect("Failed to migrate database.");
    db_pool
}

async fn spawn_app() -> TestApp {
    Lazy::force(&TRACING);
    let listener =
        TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port.");
    let port = listener
        .local_addr()
        .expect("Failed to get local address.")
        .port();
    let mut config = get_configuration().expect("Failed to read config.");
    config.database.database_name = Uuid::new_v4().to_string();
    let db_pool = configure_database(&config.database).await;

    let address = format!("http://127.0.0.1:{}", port);
    let server =
        run(listener, db_pool.clone()).expect("Failed to bind address");
    let _ = tokio::spawn(server);
    TestApp { address, db_pool }
}

#[tokio::test]
async fn health_check_works() {
    let address = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .get(&format!("{}/health_check", &address.address))
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

#[tokio::test]
async fn subscribe_returns_200_for_valid_form_data() {
    let app_address = spawn_app().await;
    let client = reqwest::Client::new();

    let body = serde_urlencoded::to_string(vec![
        ("name", "pally p"),
        ("email", "pallyp@gmail.com"),
    ])
    .unwrap();
    let response = client
        .post(&format!("{}/subscriptions", &app_address.address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request.");

    let saved = sqlx::query!("SELECT email, name FROM subscriptions")
        .fetch_one(&app_address.db_pool)
        .await
        .expect("Failed to fetch saved subscription.");

    assert_eq!(200, response.status().as_u16());
    assert_eq!(saved.email, "pallyp@gmail.com");
    assert_eq!(saved.name, "pally p");
}

#[tokio::test]
async fn subscribe_returns_400_for_missing_data() {
    let app_address = spawn_app().await;
    let client = reqwest::Client::new();
    let test_cases = vec![
        (
            serde_urlencoded::to_string(&[("name", "pallyp")]).unwrap(),
            "missing the email",
        ),
        (
            serde_urlencoded::to_string(&[("email", "pallyp@gmail.com")])
                .unwrap(),
            "missing the name",
        ),
        (String::new(), "missing both the name and the email"),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = client
            .post(&format!("{}/subscriptions", &app_address.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(invalid_body)
            .send()
            .await
            .expect("Failed to execute request.");
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request with payload {}.",
            error_message
        );
    }
}
