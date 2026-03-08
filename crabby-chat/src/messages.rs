pub mod messages;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

struct NewUserConnection {
    user_id: Uuid,
}

#[derive(Serialize)]
pub struct Outgoing;
#[derive(Serialize, Deserialize, Debug)]
pub struct UserMessage {
    pub from: Uuid,
    pub to: Uuid,
    pub contents: String,
}
#[derive(Serialize, Deserialize)]
pub struct UserDisconnected(pub Uuid);
