use crabby_transport::{
    channel::Channel,
    codec::{JsonCodec, MsgpackCodec},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::ws::outgoing::CrabbyWsFromServer;

pub struct UserMessageDelivery {
    user_id: String,
}

impl Channel for UserMessageDelivery {
    type Message = CrabbyWsFromServer;
    type Codec = JsonCodec;

    fn channel_name() -> &'static str {
        "crabby-user-delivery"
    }

    fn subject(&self) -> String {
        format!("user.{}.delivery", self.user_id)
    }
}

impl UserMessageDelivery {
    pub fn new(user_id: &dyn AsRef<str>) -> Self {
        UserMessageDelivery {
            user_id: user_id.as_ref().to_owned(),
        }
    }
}
pub struct FanoutMessageDelivery;

impl Channel for FanoutMessageDelivery {
    type Message = CrabbyWsFromServer;

    type Codec = JsonCodec;

    fn channel_name() -> &'static str {
        "crabby-fanout-delivery"
    }

    fn subject(&self) -> String {
        "messages.delivery.fanout".to_owned()
    }
}
//Temp location of this type
#[derive(Serialize, Deserialize)]
pub struct GroupChangeId(pub Uuid);
pub struct GroupChangeEvent;

impl Channel for GroupChangeEvent {
    type Message = GroupChangeId;

    type Codec = MsgpackCodec;

    fn channel_name() -> &'static str {
        "group-membership-change"
    }

    fn subject(&self) -> String {
        "groups.events".to_string()
    }
}

#[cfg(test)]
mod tests {
    use crabby_transport::{channel::Channel, codec::Codec};
    use uuid::Uuid;

    use super::*;

    #[test]
    fn channel_name_is_correct() {
        assert_eq!(UserMessageDelivery::channel_name(), "crabby-user-delivery");
    }

    #[test]
    fn subject_contains_user_id() {
        let delivery = UserMessageDelivery::new(&"user-123");
        assert_eq!(delivery.subject(), "user.user-123.delivery");
    }

    #[test]
    fn subject_varies_by_user() {
        let a = UserMessageDelivery::new(&"alice");
        let b = UserMessageDelivery::new(&"bob");
        assert_ne!(a.subject(), b.subject());
    }

    #[test]
    fn new_accepts_string_and_str() {
        let from_str = UserMessageDelivery::new(&"abc");
        let owned = String::from("abc");
        let from_string = UserMessageDelivery::new(&owned);
        assert_eq!(from_str.subject(), from_string.subject());
    }

    #[test]
    fn codec_roundtrip_chat_message() {
        let msg = CrabbyWsFromServer::ChatMessage {
            message_id: 42,
            user_id: Uuid::nil(),
            dest: crate::ws::common::Destination::Individual {
                id: Uuid::nil(),
            },
            timestamp: "2026-01-01T00:00:00Z".to_string(),
            contents: "hello".to_string(),
        };

        let encoded = JsonCodec::encode(&msg).expect("encode failed");
        let decoded: CrabbyWsFromServer =
            JsonCodec::decode(&encoded).expect("decode failed");

        match decoded {
            CrabbyWsFromServer::ChatMessage {
                message_id,
                contents,
                ..
            } => {
                assert_eq!(message_id, 42);
                assert_eq!(contents, "hello");
            }
        }
    }
}
