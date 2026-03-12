use axum::{body::Bytes, extract::ws::Message};
use crabby_specs::ws::outgoing::CrabbyWsFromServer;
use eyre::Result;

pub trait Encode<I> {
    type Output;
    fn encode(item: I) -> Result<Self::Output>;
}
pub struct ServerToTransport;

impl Encode<CrabbyWsFromServer> for ServerToTransport {
    type Output = Message;

    fn encode(item: CrabbyWsFromServer) -> Result<Self::Output> {
        let serialized = serde_json::to_vec(&item).unwrap();
        let bytes =
            Message::Binary(Bytes::copy_from_slice(serialized.as_slice()));
        Ok(bytes)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crabby_specs::ws::common::Destination;
    use uuid::Uuid;

    fn sample_server_message() -> CrabbyWsFromServer {
        CrabbyWsFromServer::ChatMessage {
            message_id: 42,
            user_id: Uuid::nil(),
            dest: Destination::Individual { id: Uuid::nil() },
            timestamp: "2026-01-01T00:00:00Z".to_string(),
            contents: "hello world".to_string(),
        }
    }

    #[test]
    fn encode_produces_binary_message() {
        let msg = sample_server_message();
        let result = ServerToTransport::encode(msg);
        assert!(result.is_ok());
        match result.unwrap() {
            Message::Binary(_) => {} // expected
            other => panic!("Expected Binary message, got {:?}", other),
        }
    }

    #[test]
    fn encode_roundtrip_preserves_contents() {
        let msg = sample_server_message();
        let encoded = ServerToTransport::encode(msg.clone()).unwrap();
        let bytes = match encoded {
            Message::Binary(b) => b,
            _ => panic!("Expected Binary"),
        };
        let decoded: CrabbyWsFromServer =
            serde_json::from_slice(&bytes).unwrap();
        match decoded {
            CrabbyWsFromServer::ChatMessage {
                message_id,
                contents,
                ..
            } => {
                assert_eq!(message_id, 42);
                assert_eq!(contents, "hello world");
            }
        }
    }

    #[test]
    fn encode_preserves_destination_individual() {
        let dest_id = Uuid::from_u128(999);
        let msg = CrabbyWsFromServer::ChatMessage {
            message_id: 1,
            user_id: Uuid::nil(),
            dest: Destination::Individual { id: dest_id },
            timestamp: String::new(),
            contents: String::new(),
        };
        let encoded = ServerToTransport::encode(msg).unwrap();
        let bytes = match encoded {
            Message::Binary(b) => b,
            _ => panic!("Expected Binary"),
        };
        let decoded: CrabbyWsFromServer =
            serde_json::from_slice(&bytes).unwrap();
        match decoded {
            CrabbyWsFromServer::ChatMessage { dest, .. } => match dest {
                Destination::Individual { id } => assert_eq!(id, dest_id),
                _ => panic!("Expected Individual destination"),
            },
        }
    }

    #[test]
    fn encode_preserves_destination_group() {
        let group_id = Uuid::from_u128(123);
        let msg = CrabbyWsFromServer::ChatMessage {
            message_id: 1,
            user_id: Uuid::nil(),
            dest: Destination::Group { id: group_id },
            timestamp: String::new(),
            contents: String::new(),
        };
        let encoded = ServerToTransport::encode(msg).unwrap();
        let bytes = match encoded {
            Message::Binary(b) => b,
            _ => panic!("Expected Binary"),
        };
        let decoded: CrabbyWsFromServer =
            serde_json::from_slice(&bytes).unwrap();
        match decoded {
            CrabbyWsFromServer::ChatMessage { dest, .. } => match dest {
                Destination::Group { id } => assert_eq!(id, group_id),
                _ => panic!("Expected Group destination"),
            },
        }
    }

    #[test]
    fn encode_empty_contents() {
        let msg = CrabbyWsFromServer::ChatMessage {
            message_id: 0,
            user_id: Uuid::nil(),
            dest: Destination::Individual { id: Uuid::nil() },
            timestamp: String::new(),
            contents: String::new(),
        };
        let result = ServerToTransport::encode(msg);
        assert!(result.is_ok());
    }

    #[test]
    fn encode_unicode_contents() {
        let msg = CrabbyWsFromServer::ChatMessage {
            message_id: 1,
            user_id: Uuid::nil(),
            dest: Destination::Individual { id: Uuid::nil() },
            timestamp: String::new(),
            contents: "🦀 héllo wörld 你好".to_string(),
        };
        let encoded = ServerToTransport::encode(msg).unwrap();
        let bytes = match encoded {
            Message::Binary(b) => b,
            _ => panic!("Expected Binary"),
        };
        let decoded: CrabbyWsFromServer =
            serde_json::from_slice(&bytes).unwrap();
        match decoded {
            CrabbyWsFromServer::ChatMessage { contents, .. } => {
                assert_eq!(contents, "🦀 héllo wörld 你好");
            }
        }
    }
}
