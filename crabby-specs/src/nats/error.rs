use async_nats::StatusCode;
#[derive(thiserror::Error, Debug)]
pub enum NatsAdapterError {
    #[error("protocol error")]
    ProtocolError {
        status: StatusCode,
        description: Option<String>,
    },
    #[error("Mismatched subject")]
    SubjectMismatch { expected: String, got: String },
    #[error("Unexpected reply")]
    UnexpectedReply,
}
