use serde_json::error;
use thiserror::Error;
use tokio::sync::mpsc::error::SendError;

use crate::event::ServerEvent;
#[derive(Debug, Error)]
pub(crate) enum ChatError {
    #[error("User was never connected")]
    UserNotConnected,
    #[error("Send channel has been replaced")]
    UserSinkReplaced,
    #[error("send channel was dropped")]
    FailedSend(#[from] SendError<ServerEvent>),
}
