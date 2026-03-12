use axum::extract::ws::{Message as WsMessage, WebSocket};
use crabby_specs::ws::incoming::CrabbyWsFromClient;
use eyre::Ok;
use futures::{Stream, stream::SplitStream};
use kameo::{
    Actor, actor::ActorRef, error::Infallible, message::StreamMessage,
    prelude::Message,
};
use std::{marker::PhantomData, pin::Pin};
use uuid::Uuid;

use crate::actors::{converter::incoming::Decode, engine::EngineActor};
//Because axum's Websocket stream returns Result<Item,Error> I need to filter_map to get a stream
//of only Items
pub type IncomingWebsocketActor = IncomingMessageActor<
    WsMessage,
    Pin<Box<dyn Stream<Item = WsMessage> + Send>>,
>;
pub struct IncomingMessageActor<I, S>
where
    S: Stream<Item = I> + Send + 'static,
    I: Send + Sync + 'static,
{
    engine: ActorRef<EngineActor>,
    user_id: Uuid,
    me: Option<ActorRef<Self>>,
    _stream: PhantomData<S>,
    _stream_item: PhantomData<I>,
}

impl<I, S> IncomingMessageActor<I, S>
where
    S: Stream<Item = I> + Send + 'static,
    I: Send + Sync + 'static,
{
    pub fn new(engine: ActorRef<EngineActor>, user_id: Uuid) -> Self {
        Self {
            engine,
            user_id,
            me: None,
            _stream: PhantomData::default(),
            _stream_item: PhantomData::default(),
        }
    }
    fn actor_ref(&mut self, handle: ActorRef<Self>) {
        self.me = Some(handle);
    }
}
impl<I, S> Actor for IncomingMessageActor<I, S>
where
    S: Stream<Item = I> + Send + 'static,
    I: Send + Sync + 'static,
{
    type Args = Self;

    type Error = Infallible;

    async fn on_start(
        mut args: Self::Args,
        actor_ref: ActorRef<Self>,
    ) -> Result<Self, Self::Error> {
        args.actor_ref(actor_ref);
        std::result::Result::Ok(args)
    }
}
impl<I, S> Decode<WsMessage> for IncomingMessageActor<I, S>
where
    S: Stream<Item = I> + Send + 'static,
    I: Send + Sync + 'static,
{
    type Output = CrabbyWsFromClient;

    type Error = Infallible;

    fn decode(
        item: WsMessage,
    ) -> eyre::Result<<IncomingMessageActor<I, S> as Decode<WsMessage>>::Output>
    {
        match item {
            WsMessage::Text(_) => unimplemented!(),
            WsMessage::Binary(bytes) => {
                let message: CrabbyWsFromClient =
                    serde_json::from_slice(bytes.as_ref())?;
                eyre::Ok(message)
            }
            WsMessage::Ping(bytes) => unimplemented!(),
            WsMessage::Pong(bytes) => unimplemented!(),
            WsMessage::Close(close_frame) => unimplemented!(),
        }
    }
}

impl<I, S> Message<StreamMessage<I, (), ()>> for IncomingMessageActor<I, S>
where
    S: Stream<Item = I> + Send + 'static,
    I: Send + Sync + 'static,
    Self: Decode<I, Output = CrabbyWsFromClient> + Send,
{
    type Reply = ();

    async fn handle(
        &mut self,
        msg: StreamMessage<I, (), ()>,
        ctx: &mut kameo::prelude::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        match msg {
            StreamMessage::Next(msg) => {
                let decoded = Self::decode(msg);
                if let std::result::Result::Ok(msg) = decoded {
                    println!("{:?}", msg);
                    let _ = self.engine.tell(msg).await;
                }
            }
            StreamMessage::Started(_) => (),
            //TODO: check to make sure implementation is secure
            StreamMessage::Finished(_) => {
                if let Some(r) = self.me.take()
                    && let Err(err) = r.stop_gracefully().await
                {
                    match err {
                        kameo::error::SendError::ActorNotRunning(_) => todo!(),
                        kameo::error::SendError::ActorStopped => todo!(),
                        kameo::error::SendError::MailboxFull(_) => todo!(),
                        kameo::error::SendError::HandlerError(_) => todo!(),
                        kameo::error::SendError::Timeout(_) => todo!(),
                    }
                }
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Bytes;
    use crabby_specs::ws::common::Destination;

    /// Helper: build a binary WsMessage from a CrabbyWsFromClient value
    fn make_binary_ws_message(msg: &CrabbyWsFromClient) -> WsMessage {
        let json = serde_json::to_vec(msg).unwrap();
        WsMessage::Binary(Bytes::from(json))
    }

    #[test]
    fn decode_valid_binary_user_message() {
        let user_id = Uuid::from_u128(1);
        let dest_id = Uuid::from_u128(2);
        let original = CrabbyWsFromClient::UserMessage {
            user_id,
            dest: Destination::Individual { id: dest_id },
            timestamp: "2026-03-01T12:00:00Z".to_string(),
            contents: "test message".to_string(),
        };
        let ws_msg = make_binary_ws_message(&original);
        let decoded =
            <IncomingWebsocketActor as Decode<WsMessage>>::decode(ws_msg);
        assert!(decoded.is_ok());
        match decoded.unwrap() {
            CrabbyWsFromClient::UserMessage {
                user_id: uid,
                contents,
                ..
            } => {
                assert_eq!(uid, user_id);
                assert_eq!(contents, "test message");
            }
        }
    }

    #[test]
    fn decode_invalid_binary_returns_error() {
        let ws_msg = WsMessage::Binary(Bytes::from_static(b"not valid json"));
        let decoded =
            <IncomingWebsocketActor as Decode<WsMessage>>::decode(ws_msg);
        assert!(decoded.is_err());
    }

    #[test]
    fn decode_empty_binary_returns_error() {
        let ws_msg = WsMessage::Binary(Bytes::from_static(b""));
        let decoded =
            <IncomingWebsocketActor as Decode<WsMessage>>::decode(ws_msg);
        assert!(decoded.is_err());
    }

    #[test]
    fn decode_valid_json_wrong_schema_returns_error() {
        // Valid JSON but not a CrabbyWsFromClient
        let ws_msg =
            WsMessage::Binary(Bytes::from_static(b"{\"foo\": \"bar\"}"));
        let decoded =
            <IncomingWebsocketActor as Decode<WsMessage>>::decode(ws_msg);
        assert!(decoded.is_err());
    }

    #[test]
    #[should_panic(expected = "not implemented")]
    fn decode_text_message_panics() {
        let ws_msg = WsMessage::Text("some text".into());
        let _ = <IncomingWebsocketActor as Decode<WsMessage>>::decode(ws_msg);
    }

    #[test]
    #[should_panic(expected = "not implemented")]
    fn decode_ping_panics() {
        let ws_msg = WsMessage::Ping(Bytes::from_static(b"ping"));
        let _ = <IncomingWebsocketActor as Decode<WsMessage>>::decode(ws_msg);
    }

    #[test]
    #[should_panic(expected = "not implemented")]
    fn decode_pong_panics() {
        let ws_msg = WsMessage::Pong(Bytes::from_static(b"pong"));
        let _ = <IncomingWebsocketActor as Decode<WsMessage>>::decode(ws_msg);
    }

    #[test]
    #[should_panic(expected = "not implemented")]
    fn decode_close_panics() {
        let ws_msg = WsMessage::Close(None);
        let _ = <IncomingWebsocketActor as Decode<WsMessage>>::decode(ws_msg);
    }

    #[test]
    fn decode_preserves_group_destination() {
        let group_id = Uuid::from_u128(999);
        let original = CrabbyWsFromClient::UserMessage {
            user_id: Uuid::nil(),
            dest: Destination::Group { id: group_id },
            timestamp: String::new(),
            contents: String::new(),
        };
        let ws_msg = make_binary_ws_message(&original);
        let decoded =
            <IncomingWebsocketActor as Decode<WsMessage>>::decode(ws_msg)
                .unwrap();
        match decoded {
            CrabbyWsFromClient::UserMessage { dest, .. } => match dest {
                Destination::Group { id } => assert_eq!(id, group_id),
                _ => panic!("Expected Group destination"),
            },
        }
    }

    #[test]
    fn decode_unicode_contents() {
        let original = CrabbyWsFromClient::UserMessage {
            user_id: Uuid::nil(),
            dest: Destination::Individual { id: Uuid::nil() },
            timestamp: String::new(),
            contents: "🦀 crabs are chatty 日本語".to_string(),
        };
        let ws_msg = make_binary_ws_message(&original);
        let decoded =
            <IncomingWebsocketActor as Decode<WsMessage>>::decode(ws_msg)
                .unwrap();
        match decoded {
            CrabbyWsFromClient::UserMessage { contents, .. } => {
                assert_eq!(contents, "🦀 crabs are chatty 日本語");
            }
        }
    }
}
