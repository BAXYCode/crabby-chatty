use axum::{body::Bytes, extract::ws::Message};
use eyre::Result;

use crate::messages::UserMessage;

pub trait Encode<I> {
    type Output;
    fn encode(item: I) -> Result<Self::Output>;
}
pub struct UserMessageToWsMessage;

impl Encode<UserMessage> for UserMessageToWsMessage {
    type Output = Message;

    fn encode(item: UserMessage) -> Result<Self::Output> {
        let serialized = serde_json::to_vec(&item).unwrap();
        let bytes = Message::Binary(Bytes::copy_from_slice(serialized.as_slice()));
        Ok(bytes)
    }
}
