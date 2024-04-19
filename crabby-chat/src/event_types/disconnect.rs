use crate::handle::Handler;
use crate::ChatEngine;
use crate::{error::ChatError, event::ServerEvent};
use tokio::sync::mpsc::UnboundedSender;
use tracing::{info, warn};
use uuid::Uuid;
#[derive(Debug)]
pub(crate) struct Disconnect {
    id: Uuid,
}

impl Handler<Disconnect, ()> for ChatEngine {
    async fn handle(&mut self, event: Disconnect) -> Result<(), ChatError> {
        let val = self.map.remove(&event.id);
        if let Some(val) = val {
            info!("User removed from connected users");
            return Ok(());
        }
        let err = ChatError::UserNotConnected;
        warn!("{:?}", err);
        Err(err)
    }
}
