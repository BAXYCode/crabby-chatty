pub mod messages;
use futures::Sink;
use kameo::{
    Actor,
    actor::{ActorRef, Recipient},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::actors::outgoing::OutgoingMessageActor;

pub struct UserConnected(pub Uuid, pub Recipient<UserMessage>);

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UserMessage {
    pub from: Uuid,
    pub to: Uuid,
    pub contents: String,
}
#[derive(Serialize, Deserialize)]
pub struct UserDisconnected(pub Uuid);
