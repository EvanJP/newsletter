use std::net::TcpListener;

fn spawn_app() -> String {
    let listener =
        TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port.");
    let port = listener
        .local_addr()
        .expect("Failed to get local address.")
        .port();
    let server = newsletter::run(listener).expect("Failed to bind address");
    let _ = tokio::spawn(server);
    format!("http://127.0.0.1:{}", port)
}

#[tokio::test]
async fn health_check_works() {
    let address = spawn_app();
    let client = reqwest::Client::new();

    let response = client
        .get(&format!("{}/health_check", &address))
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());
}

#[tokio::test]
async fn subscribe_returns_200_for_valid_form_data() {
    let app_address = spawn_app();
    let client = reqwest::Client::new();

    let body = serde_urlencoded::to_string(vec![
        ("name", "pally p"),
        ("email", "pallyp@gmail.com"),
    ])
    .unwrap();
    let response = client
        .post(&format!("{}/subscriptions", &app_address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request.");
    assert_eq!(200, response.status().as_u16())
}

#[tokio::test]
async fn subscribe_returns_400_for_missing_data() {
    let app_address = spawn_app();
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
            .post(&format!("{}/subscriptions", &app_address))
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
