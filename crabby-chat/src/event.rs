use crate::event_types::{
    connect::Connect, disconnect::Disconnect, messages::Message,
};
#[derive(Debug)]
pub(crate) enum ServerEvent {
    Connected(Connect),
    Disconnected(Disconnect),
    ChatMessage(Message),
}

impl From<Message> for ServerEvent {
    fn from(value: Message) -> Self {
        ServerEvent::ChatMessage(value)
    }
}
