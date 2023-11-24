use wiremock::matchers::method;
use wiremock::matchers::path;
use wiremock::Mock;
use wiremock::ResponseTemplate;

use crate::helpers::spawn_app;

#[tokio::test]
async fn subscribe_returns_200_for_valid_form_data() {
    let app = spawn_app().await;

    let body = serde_urlencoded::to_string(vec![
        ("name", "pally p"),
        ("email", "pallyp@gmail.com"),
    ])
    .unwrap();
    Mock::given(path("/send"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    let response = app.post_subscriptions(body.into()).await;

    assert_eq!(200, response.status().as_u16());
}

#[tokio::test]
async fn subscribe_persists_new_subscriber() {
    let app = spawn_app().await;
    let body = serde_urlencoded::to_string(vec![
        ("name", "pally p"),
        ("email", "pallyp@gmail.com"),
    ])
    .unwrap();

    Mock::given(path("/send"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;

    let saved = sqlx::query!("SELECT email, name, status FROM subscriptions")
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved susbcription.");

    assert_eq!(saved.email, "pallyp@gmail.com");
    assert_eq!(saved.name, "pally p");
    assert_eq!(saved.status, "pending_confirmation")
}

#[tokio::test]
async fn subscribe_returns_400_for_missing_data() {
    let app = spawn_app().await;
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
        let response = app.post_subscriptions(invalid_body.into()).await;
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not fail with 400 Bad Request with payload {}.",
            error_message
        );
    }
}

#[tokio::test]
async fn subscribe_returns_a_400_when_fields_are_invalid() {
    // Arrange
    let app = spawn_app().await;
    let test_cases = vec![
        (
            serde_urlencoded::to_string(&[
                ("name", ""),
                ("email", "test@gmail.com"),
            ])
            .unwrap(),
            "empty name",
        ),
        (
            serde_urlencoded::to_string(&[("name", "test"), ("email", "")])
                .unwrap(),
            "empty email",
        ),
        (
            serde_urlencoded::to_string(&[
                ("name", "test"),
                ("email", "notanemail"),
            ])
            .unwrap(),
            "invalid email",
        ),
    ];
    for (body, description) in test_cases {
        let response = app.post_subscriptions(body.into()).await;
        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not return 400 when payload was {}.",
            description
        );
    }
}

#[tokio::test]
async fn subscribe_sends_confirmation_email_for_valid_data() {
    let app = spawn_app().await;
    let body = serde_urlencoded::to_string(&[
        ("name", "test"),
        ("email", "test@gmail.com"),
    ])
    .unwrap();

    Mock::given(path("/send"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;
}

#[tokio::test]
async fn subscribe_sends_confirmation_email_with_link() {
    let app = spawn_app().await;
    let body = serde_urlencoded::to_string(&[
        ("name", "test"),
        ("email", "test@gmail.com"),
    ])
    .unwrap();

    Mock::given(path("/send"))
        .and(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&app.email_server)
        .await;

    app.post_subscriptions(body.into()).await;
    let email_request = &app.email_server.received_requests().await.unwrap()[0];
    let confirmation_links = app.get_confirmation_links(&email_request);
    assert_eq!(confirmation_links.html, confirmation_links.plain_text);
}

#[tokio::test]
async fn subscribe_fails_if_fatal_database_error() {
    let app = spawn_app().await;
    let body = serde_urlencoded::to_string(&[
        ("name", "test"),
        ("email", "test@gmail.com"),
    ])
    .unwrap();
    sqlx::query!(
        "ALTER TABLE subscription_tokens DROP COLUMN subscription_token;"
    )
    .execute(&app.db_pool)
    .await
    .unwrap();

    let response = app.post_subscriptions(body.into()).await;

    assert_eq!(response.status().as_u16(), 500);
}
