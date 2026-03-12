use crate::{
    actors::{
        converter::outgoing::{Encode, ServerToTransport},
        engine::EngineActor,
    },
    messages::internal::UserConnected,
};
use axum::{extract::ws::Message as WsMessage, extract::ws::WebSocket};
use crabby_specs::ws::outgoing::CrabbyWsFromServer;
use futures::{Sink, SinkExt, stream::SplitSink};
use kameo::{Actor, actor::ActorRef, error::Infallible, prelude::Message};
use std::marker::PhantomData;
use uuid::Uuid;
pub struct OutgoingMessageActor<S, I, C>
where
    S: Sink<I> + Send + Sync + 'static,
    I: Send + Sync + 'static,
    C: Encode<CrabbyWsFromServer, Output = I> + Send + Sync + 'static,
{
    sink: S,
    _phantom: PhantomData<I>,
    engine: ActorRef<EngineActor>,
    converter: C,
    user_id: Uuid,
}
pub type OutgoingWebsocketActor = OutgoingMessageActor<
    SplitSink<WebSocket, WsMessage>,
    WsMessage,
    ServerToTransport,
>;

impl
    OutgoingMessageActor<
        SplitSink<WebSocket, WsMessage>,
        WsMessage,
        ServerToTransport,
    >
{
    pub fn new(
        sink: SplitSink<WebSocket, WsMessage>,
        engine_ref: ActorRef<EngineActor>,
        user_id: Uuid,
    ) -> Self {
        Self {
            sink,
            engine: engine_ref,
            converter: ServerToTransport,
            user_id,
            _phantom: PhantomData,
        }
    }
}

impl<S, I, C> Actor for OutgoingMessageActor<S, I, C>
where
    S: SinkExt<I> + Send + Sync + 'static + futures::Sink<I> + Unpin,
    I: Send + Sync + 'static,
    C: Encode<CrabbyWsFromServer, Output = I> + Send + Sync,
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

impl<S, I, C> Message<CrabbyWsFromServer> for OutgoingMessageActor<S, I, C>
where
    S: SinkExt<I> + Send + Sync + 'static + futures::Sink<I> + Unpin,
    I: Send + Sync + 'static,
    C: Encode<CrabbyWsFromServer, Output = I> + Send + Sync,
{
    type Reply = ();

    async fn handle(
        &mut self,
        msg: CrabbyWsFromServer,
        _ctx: &mut kameo::prelude::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        if let Ok(encoded) = C::encode(msg) {
            let _ = self.sink.send(encoded).await;
        }
    }
}
