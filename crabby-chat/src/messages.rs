use crate::handle::Handler;
use crate::ChatEngine;
use crabby_core::engine::Engine;
use futures::future::AndThen;
use serde::{Deserialize, Serialize};
use tracing::info;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    content: MessageContent,
    // id: Uuid,
    from: Uuid,
    // destination: Uuid,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) enum MessageContent {
    String(String),
    File(Vec<u8>),
    Photo(Vec<u8>),
}

impl Handler<Message, ()> for ChatEngine {
    async fn handle(&mut self, event: Message) -> Result<(), crate::error::ChatError> {
        let origin = event.from;
        info!("message received inside handler: {:?}", event);
        for (k, v) in self.map.iter() {
            // if &origin != k {
            info!(
                "sending message to client handler, {:?} , {:?}",
                event.from, event.content
            );
            let _res = v.send(event.clone());
            // }
        }
        Ok(())
    }
}
