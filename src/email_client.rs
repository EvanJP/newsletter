//! The email client for the newsletter, using SendGrid's API.
use std::collections::HashMap;

use reqwest::Client;
use reqwest::Url;
use secrecy::ExposeSecret;
use secrecy::Secret;
use serde::Serialize;

use crate::domain::SubscriberEmail;

/// Maps the `type` to the content in `value`.
#[derive(Serialize)]
struct EmailContent {
    r#type: String,
    value: String,
}

/// The body for [sendgrid's mail-send](https://docs.sendgrid.com/api-reference/mail-send/mail-send)
///
/// * `from` - Maps from `email` to `name`.
/// * `personalizations` - Maps [personalizations](https://docs.sendgrid.com/for-developers/sending-email/personalizations),
///   such as `to`: `email`.
/// * `subject` - The email subject.
/// * `content` - A Vector holding the plain text and HTML text
///   [`EmailContent`].
#[derive(Serialize)]
struct SendEmailRequest {
    // email: String, name: String
    from: HashMap<String, String>,
    personalizations: Vec<HashMap<String, String>>,
    subject: String,
    content: Vec<EmailContent>,
}

/// The EmailClient for newsletter delivery.
///
/// The fields are private, but are initialized in the public `new` function.
///
/// * `base_url` - Holds the app's API url.
/// * `http_client` - A [`reqwest::Client`] to send requests through.
/// * `sender` - The [`SubscriberEmail`] that represents the sender.
/// * `authorization_token` - A [`secrecy::Secret`] String that holds our
///   sendgrid API key.
pub struct EmailClient {
    base_url: Url,
    http_client: Client,
    sender: SubscriberEmail,
    authorization_token: Secret<String>,
}

impl EmailClient {
    /// Initializes the `EmailClient` according to the struct fields.
    /// Additionally holds a `timeout` for the [`reqwest::Client`] to use.
    pub fn new(
        base_url: Url,
        sender: SubscriberEmail,
        authorization_token: Secret<String>,
        timeout: std::time::Duration,
    ) -> Self {
        let http_client = Client::builder().timeout(timeout).build().unwrap();
        Self {
            http_client,
            base_url,
            sender,
            authorization_token,
        }
    }

    /// Sends the email through the [SendGrid API](https://docs.sendgrid.com/api-reference/mail-send/mail-send).
    ///
    /// Sends through the initialized [`reqwest::Client`].
    ///
    /// # Arguments
    ///
    /// * `recipient` - Who the email should be sent to.
    /// * `subject` - The email subject.
    /// * `html_content` - Content in HTML form.
    /// * `text_content` - Content in plain-text.
    pub async fn send_email(
        &self,
        recipient: &SubscriberEmail,
        subject: &str,
        html_content: &str,
        text_content: &str,
    ) -> Result<(), reqwest::Error> {
        let url = self.base_url.join("send").expect("Failed to join urls.");
        let request_body = SendEmailRequest {
            from: HashMap::from([(
                "email".to_string(),
                self.sender.as_ref().into(),
            )]),
            personalizations: Vec::from([HashMap::from([(
                "email".to_string(),
                recipient.as_ref().to_owned(),
            )])]),
            subject: subject.to_owned(),
            content: Vec::from([
                EmailContent {
                    r#type: "text/plain".to_string(),
                    value: text_content.to_string(),
                },
                EmailContent {
                    r#type: "text/html".to_string(),
                    value: html_content.to_string(),
                },
            ]),
        };
        self.http_client
            .post(url)
            .bearer_auth(self.authorization_token.expose_secret())
            .json(&request_body)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use claym::assert_err;
    use claym::assert_ok;
    use fake::faker::internet::en::SafeEmail;
    use fake::faker::lorem::en::Paragraph;
    use fake::faker::lorem::en::Sentence;
    use fake::Fake;
    use fake::Faker;
    use secrecy::Secret;
    use url::Url;
    use wiremock::matchers::any;
    use wiremock::matchers::header;
    use wiremock::matchers::header_exists;
    use wiremock::matchers::method;
    use wiremock::matchers::path;
    use wiremock::Mock;
    use wiremock::MockServer;
    use wiremock::Request;
    use wiremock::ResponseTemplate;

    use crate::domain::SubscriberEmail;
    use crate::email_client::EmailClient;

    struct SendEmailBodyMatcher;

    impl wiremock::Match for SendEmailBodyMatcher {
        fn matches(&self, request: &Request) -> bool {
            let result: Result<serde_json::Value, _> =
                serde_json::from_slice(&request.body);
            if let Ok(body) = result {
                body.get("from").is_some()
                    && body.get("personalizations").is_some()
                    && body.get("subject").is_some()
                    && body.get("content").is_some()
            } else {
                false
            }
        }
    }

    fn subject() -> String {
        Sentence(1..2).fake()
    }

    fn content() -> String {
        Paragraph(1..10).fake()
    }

    fn email() -> SubscriberEmail {
        SubscriberEmail::parse(SafeEmail().fake()).unwrap()
    }

    fn email_client(base_url: Url) -> EmailClient {
        EmailClient::new(
            base_url,
            email(),
            Secret::new(Faker.fake()),
            std::time::Duration::from_millis(200),
        )
    }

    #[tokio::test]
    async fn send_email_succeeds_if_server_returns_200() {
        let mock_server = MockServer::start().await;
        let mock_server_uri = url::Url::parse(mock_server.uri().as_str())
            .expect("Failed to parse URI.");
        let email_client = email_client(mock_server_uri);

        Mock::given(header_exists("Authorization"))
            .and(header("Content-Type", "application/json"))
            .and(path("/send"))
            .and(method("POST"))
            .and(SendEmailBodyMatcher)
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let outcome = email_client
            .send_email(&email(), &subject(), &content(), &content())
            .await;

        assert_ok!(outcome);
    }

    #[tokio::test]
    async fn send_email_fails_if_server_returns_500() {
        let mock_server = MockServer::start().await;
        let mock_server_uri = url::Url::parse(mock_server.uri().as_str())
            .expect("Failed to parse URI.");
        let email_client = email_client(mock_server_uri);

        Mock::given(any())
            .respond_with(ResponseTemplate::new(500))
            .expect(1)
            .mount(&mock_server)
            .await;

        let outcome = email_client
            .send_email(&email(), &subject(), &content(), &content())
            .await;

        assert_err!(outcome);
    }

    #[tokio::test]
    async fn send_email_times_out_if_server_takes_too_long() {
        let mock_server = MockServer::start().await;
        let mock_server_uri = url::Url::parse(mock_server.uri().as_str())
            .expect("Failed to parse URI.");
        let email_client = email_client(mock_server_uri);

        Mock::given(any())
            .respond_with(
                ResponseTemplate::new(200)
                    .set_delay(std::time::Duration::from_secs(10)),
            )
            .expect(1)
            .mount(&mock_server)
            .await;

        let outcome = email_client
            .send_email(&email(), &subject(), &content(), &content())
            .await;

        assert_err!(outcome);
    }
}
