use ::serde::{Deserialize, Serialize};
use asyncapi_rust::{ToAsyncApiMessage, schemars::JsonSchema};
use uuid::Uuid;

use crate::ws::common::Destination;

//Any other type of websocket message that I will be sending back to the client will be defined
//inside of this enum
#[derive(
    Debug, Clone, JsonSchema, ToAsyncApiMessage, Serialize, Deserialize,
)]
#[serde(tag = "type")]
pub enum CrabbyWsFromServer {
    #[asyncapi(description = "Server sent chat message")]
    ChatMessage {
        message_id: u64,
        user_id: Uuid,
        dest: Destination,
        timestamp: String,
        contents: String,
    },
}
