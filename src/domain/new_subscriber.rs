use serde::Serialize;

use crate::domain::SubscriberEmail;
use crate::domain::SubscriberName;

#[derive(Debug, Serialize)]
pub struct NewSubscriber {
    pub email: SubscriberEmail,
    pub name: SubscriberName,
}
