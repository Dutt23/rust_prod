#[derive(Debug)]
pub struct SubscriberEmail(String);
use validator::validate_email;

impl SubscriberEmail {
    pub fn parse(email: String) -> Result<SubscriberEmail, String> {
        if validate_email(&email) {
            Ok(Self(email))
        } else {
            Err(format!("{} is not a valid email", email))
        }
    }
}

impl AsRef<str> for SubscriberEmail {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use crate::domains::SubscriberEmail;
    use claims::assert_err;

    #[test]
    fn empty_string_is_rejected() {
        let email = "".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }

    #[test]
    fn reject_email_with_missing_at_symbol() {
        let email = "shatya.com".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }

    #[test]
    fn reject_email_with_subject_missing() {
        let email = "domain.com".to_string();
        assert_err!(SubscriberEmail::parse(email));
    }
}
