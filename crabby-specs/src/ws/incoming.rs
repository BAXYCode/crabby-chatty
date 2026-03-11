use ::serde::{Deserialize, Serialize};
use asyncapi_rust::{ToAsyncApiMessage, schemars::JsonSchema};
use uuid::Uuid;

use crate::ws::common::Destination;

#[derive(JsonSchema, ToAsyncApiMessage, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CrabbyWsFromClient {
    #[asyncapi(description = "User sent chat message")]
    UserMessage {
        user_id: Uuid,
        dest: Destination,
        timestamp: String,
        contents: String,
    },
}
