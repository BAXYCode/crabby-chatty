use crate::{
    actors::{
        converter::outgoing::{Encode, UserMessageToWsMessage},
        engine::EngineActor,
    },
    messages::{UserConnected, UserMessage},
};
use axum::{body::Bytes, extract::ws::Message as WsMessage, extract::ws::WebSocket};
use eyre::Ok as EyreOk;
use futures::{Sink, SinkExt, Stream, stream::SplitSink};
use kameo::{Actor, actor::ActorRef, error::Infallible, prelude::Message};
use serde::Serialize;
use std::{marker::PhantomData, process::Output};
use uuid::Uuid;
pub struct OutgoingMessageActor<S, I, C>
where
    S: Sink<I> + Send + Sync + 'static,
    I: Send + Sync + 'static,
    C: Encode<UserMessage, Output = I> + Send + Sync + 'static,
{
    sink: S,
    _phantom: PhantomData<I>,
    engine: ActorRef<EngineActor>,
    converter: C,
    user_id: Uuid,
}
pub type OutgoingWebsocketActor =
    OutgoingMessageActor<SplitSink<WebSocket, WsMessage>, WsMessage, UserMessageToWsMessage>;

impl OutgoingMessageActor<SplitSink<WebSocket, WsMessage>, WsMessage, UserMessageToWsMessage> {
    pub fn new(
        sink: SplitSink<WebSocket, WsMessage>,
        engine_ref: ActorRef<EngineActor>,
        user_id: Uuid,
    ) -> Self {
        Self {
            sink,
            engine: engine_ref,
            converter: UserMessageToWsMessage,
            user_id,
            _phantom: PhantomData::default(),
        }
    }
}

impl<S, I, C> Actor for OutgoingMessageActor<S, I, C>
where
    S: SinkExt<I> + Send + Sync + 'static + futures::Sink<I> + Unpin,
    I: Send + Sync + 'static,
    C: Encode<UserMessage, Output = I> + Send + Sync,
{
    type Args = Self;

    type Error = Infallible;

    async fn on_start(
        args: Self::Args,
        actor_ref: kameo::prelude::ActorRef<Self>,
    ) -> Result<Self, Self::Error> {
        let _ = args
            .engine
            .tell(UserConnected(args.user_id, actor_ref.clone().recipient()))
            .await;
        Ok(args)
    }
}
// impl<S, C> Encode<UserMessage> for OutgoingMessageActor<S, WsMessage, C>
// where
//     S: Send + Sync + 'static + Sink<WsMessage> + Unpin,
//     C: Encode<UserMessage, Output = WsMessage> + Send + Sync,
// {
//     type Output = WsMessage;
//
//     fn encode(item: UserMessage) -> eyre::Result<Self::Output> {
//         //TODO: very bad error handling
//         let serialized = serde_json::to_vec(&item).unwrap();
//         let bytes = WsMessage::Binary(Bytes::copy_from_slice(serialized.as_slice()));
//         EyreOk(bytes)
//     }
// }

impl<S, I, C> Message<UserMessage> for OutgoingMessageActor<S, I, C>
where
    S: SinkExt<I> + Send + Sync + 'static + futures::Sink<I> + Unpin,
    I: Send + Sync + 'static,
    C: Encode<UserMessage, Output = I> + Send + Sync,
{
    type Reply = ();

    async fn handle(
        &mut self,
        msg: UserMessage,
        ctx: &mut kameo::prelude::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        println!("inside outgoing before encode");
        if let Ok(encoded) = C::encode(msg) {
            println!("inside outgoing before sink");
            let _ = self.sink.send(encoded).await;
        }
    }
}
