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
    use fake::{faker::internet::en::SafeEmail, Fake};
    use rand::{rngs::StdRng, SeedableRng};

    #[derive(Debug, Clone)]
    pub struct ValidationEmailFixture(pub String);

    impl quickcheck::Arbitrary for ValidationEmailFixture {
        fn arbitrary(g: &mut quickcheck::Gen) -> Self {
            let mut rng = StdRng::seed_from_u64(u64::arbitrary(g));
            let email = SafeEmail().fake_with_rng(&mut rng);
            Self(email)
        }
    }

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

    #[quickcheck_macros::quickcheck]
    fn valid_emails(valid_email: ValidationEmailFixture) -> bool {
        SubscriberEmail::parse(valid_email.0).is_ok()
    }
}
