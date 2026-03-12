use serde_json::error;
use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum ChatError {
    #[error("User was never connected")]
    UserNotConnected,
    #[error("Send channel has been replaced")]
    UserSinkReplaced,
}
