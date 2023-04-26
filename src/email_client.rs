use crate::domains::SubscriberEmail;
use reqwest::Client;
use serde::{Deserialize, Serialize};

pub struct EmailClient {
    client: Client,
    base_url: String,
    sender_email: SubscriberEmail,
}

#[derive(Serialize, Deserialize, Debug)]
struct SendEmailRequest {
    from: String,
    to: String,
    subject: String,
    html_body: String,
    text_body: String,
}

impl EmailClient {
    pub fn new(base_url: String, sender_email: SubscriberEmail) -> Self {
        Self {
            client: Client::new(),
            base_url,
            sender_email,
        }
    }

    pub async fn send_email(
        &self,
        recipient: SubscriberEmail,
        subject: &str,
        html_content: &str,
        text_content: &str,
    ) -> Result<(), String> {
        let url = format!("{}/email", self.base_url);
        let request_body = SendEmailRequest {
            to: recipient.as_ref().to_owned(),
            from: self.sender_email.as_ref().to_owned(),
            subject: subject.to_string(),
            html_body: html_content.to_string(),
            text_body: text_content.to_string(),
        };

        let builder = self.client.post(url).json(&request_body);
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use crate::email_client::EmailClient;
    use fake::faker::internet::en::SafeEmail;
    use fake::faker::lorem::en::{Paragraph, Sentence};
    use fake::{Fake, Faker};
    use wiremock::{matchers::any, Mock, MockServer, ResponseTemplate};

    use crate::domains::SubscriberEmail;

    #[tokio::test]
    async fn send_email_fires_a_request_to_base_url() {
        let mock_server = MockServer::start().await;
        let sender = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let email_client = EmailClient::new(mock_server.uri(), sender);

        Mock::given(any())
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&mock_server)
            .await;

        let subscriber_email = SubscriberEmail::parse(SafeEmail().fake()).unwrap();
        let subject: String = Sentence(1..2).fake();
        let paragraph: String = Paragraph(1..20).fake();

        let _ = email_client
            .send_email(subscriber_email, &subject, &paragraph, &paragraph)
            .await;
    }
}
