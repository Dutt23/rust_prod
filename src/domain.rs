pub struct SubscriberName(String);
use unicode_segmentation::UnicodeSegmentation;

pub struct NewSubscriber {
    pub email: String,
    pub name: SubscriberName,
}

impl SubscriberName {
    pub fn parse(name: String) -> SubscriberName {
        let is_empty_string = name.trim().is_empty();
        let is_too_long = name.graphemes(true).count() > 256;
        let forbidden_chars = ['/', '(', ')', '"', '<', '>', '\\', '{', '}'];
        let forbidden_char_present = name.chars().any(|char| forbidden_chars.contains(&char));

        if is_empty_string || is_too_long || forbidden_char_present {
            panic!("{} is not a valid subscriber name", name)
        } else {
            Self(name)
        }
    }
}

impl AsRef<str> for SubscriberName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}
