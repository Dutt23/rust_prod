use crate::domains::SubscriberEmail;
use reqwest::Client;
pub struct EmailClient {
    client: Client,
    base_url: String,
    sender_email: SubscriberEmail,
}

impl EmailClient {
    pub fn new(base_url: String, sender_email: SubscriberEmail) -> Self {
        Self {
            client: Client::new(),
            base_url,
            sender_email,
        }
    }

    pub fn send_email(
        &self,
        recipient: SubscriberEmail,
        subject: &str,
        html_content: &str,
        text_content: &str,
    ) -> Result<(), String> {
        todo!()
    }
}
