use crate::domains::SubscriberEmail;
use crate::domains::SubscriberName;

pub struct NewSubscriber {
    pub email: SubscriberEmail,
    pub name: SubscriberName,
}
