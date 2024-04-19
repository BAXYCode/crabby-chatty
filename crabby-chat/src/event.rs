use crate::{
    event_types::{connect::Connect, disconnect::Disconnect},
    messages::Message,
};
use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;
#[derive(Debug)]
pub(crate) enum ServerEvent {
    Connected(Connect),
    Disconnected(Disconnect),
    ChatMessage(Message),
}
