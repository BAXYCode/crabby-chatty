use crate::error::ChatError;
use crate::event::ServerEvent;
use crate::handle;
use crate::handle::Handler;
use crate::ChatEngine;
use crate::ChatMessage;
use futures::io::sink;
use log::warn;
use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;

use super::disconnect::Disconnect;
#[derive(Debug, Clone)]
pub(crate) struct Connect {
    id: Uuid,
    sink: UnboundedSender<ChatMessage>,
}
impl Connect {
    pub fn new(send: UnboundedSender<ChatMessage>, id: Uuid) -> Self {
        Self { sink: send, id }
    }
}

impl Handler<Connect, ()> for ChatEngine {
    async fn handle(&mut self, event: Connect) -> Result<(), ChatError> {
        let res = self.map.insert(event.id, event.sink);
        if let Some(val) = res {
            warn!("channel replaced for user_id: {:?}", event.id);
            return Err(ChatError::UserSinkReplaced);
        }
        Ok(())
    }
}
