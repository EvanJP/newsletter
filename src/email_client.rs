use std::collections::HashMap;

use reqwest::Client;
use reqwest::Url;
use secrecy::ExposeSecret;
use secrecy::Secret;
use serde::Serialize;

use crate::domain::SubscriberEmail;

#[derive(Serialize)]
struct EmailContent {
    // We need to escape `type`.
    r#type: String,
    value: String,
}

// https://docs.sendgrid.com/api-reference/mail-send/mail-send
#[derive(Serialize)]
struct SendEmailRequest {
    // email: String, name: String
    from: HashMap<String, String>,
    personalizations: Vec<HashMap<String, String>>,
    subject: String,
    content: Vec<EmailContent>,
}

pub struct EmailClient {
    base_url: Url,
    http_client: Client,
    sender: SubscriberEmail,
    authorization_token: Secret<String>,
}

impl EmailClient {
    pub fn new(
        base_url: Url,
        sender: SubscriberEmail,
        authorization_token: Secret<String>,
    ) -> Self {
        Self {
            http_client: Client::new(),
            base_url,
            sender,
            authorization_token,
        }
    }

    pub async fn send_email(
        &self,
        recipient: SubscriberEmail,
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
        let builder = self
            .http_client
            .post(url)
            .bearer_auth(self.authorization_token.expose_secret())
            .json(&request_body)
            .send()
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use fake::faker::internet::en::SafeEmail;
    use fake::faker::lorem::en::Paragraph;
    use fake::faker::lorem::en::Sentence;
    use fake::Fake;
    use fake::Faker;
    use secrecy::Secret;
    use wiremock::matchers::header;
    use wiremock::matchers::header_exists;
    use wiremock::matchers::method;
    use wiremock::matchers::path;
    use wiremock::Mock;
    use wiremock::MockServer;
    use wiremock::ResponseTemplate;

    use crate::domain::SubscriberEmail;
    use crate::email_client::EmailClient;

    #[tokio::test]
    async fn send_email_fires_a_request_to_base_url() {
        let mock_server = MockServer::start().await;
        let sender = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        println!("the mock server uri: {:?}", mock_server.uri());
        let mock_server_uri = url::Url::parse(mock_server.uri().as_str())
            .expect("Failed to parse URI.");
        let email_client = EmailClient::new(
            mock_server_uri,
            sender,
            Secret::new(Faker.fake()),
        );

        Mock::given(header_exists("Authorization"))
            .and(header("Content-Type", "application/json"))
            .and(path("/send"))
            .and(method("POST"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let subscriber_email =
            SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let subject: String = Sentence(1..2).fake();
        let content: String = Paragraph(1..10).fake();

        let _ = email_client
            .send_email(subscriber_email, &subject, &content, &content)
            .await;
    }
}
