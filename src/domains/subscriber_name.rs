#[derive(Debug)]
pub struct SubscriberName(String);
use unicode_segmentation::UnicodeSegmentation;

impl SubscriberName {
    pub fn parse(name: String) -> Result<SubscriberName, String> {
        let is_empty_string = name.trim().is_empty();
        let is_too_long = name.graphemes(true).count() > 256;
        let forbidden_chars = ['/', '(', ')', '"', '<', '>', '\\', '{', '}'];
        let forbidden_char_present = name.chars().any(|char| forbidden_chars.contains(&char));

        if is_empty_string || is_too_long || forbidden_char_present {
            Err(format!("{} is not a valid subscriber name", name))
        } else {
            Ok(Self(name))
        }
    }
}

impl AsRef<str> for SubscriberName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use crate::domains::SubscriberName;
    use claims::{assert_err, assert_ok};

    #[test]
    fn a_256_graphene_long_name_is_valid() {
        let name = "a".repeat(256);
        assert_ok!(SubscriberName::parse(name));
    }

    #[test]
    fn a_name_longer_then_256_is_rejected() {
        let name = "a".repeat(257);
        assert_err!(SubscriberName::parse(name));
    }

    #[test]
    fn white_spaced_names_are_rejected() {
        let name = " ".to_string();
        assert_err!(SubscriberName::parse(name));
    }

    #[test]
    fn names_containing_invalid_character_rejected() {
        for name in &['/', '(', ')', '"', '<', '>', '\\', '{', '}'] {
            assert_err!(SubscriberName::parse(name.to_string()));
        }
    }

    #[test]
    fn valid_name_parsed_successfully() {
        let name = "Valid name".to_string();
        assert_ok!(SubscriberName::parse(name.to_string()));
    }
}
