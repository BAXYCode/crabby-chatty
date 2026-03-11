use axum::extract::ws::{Message as WsMessage, WebSocket};
use eyre::Ok;
use futures::{Stream, stream::SplitStream};
use kameo::{
    Actor, actor::ActorRef, error::Infallible, message::StreamMessage,
    prelude::Message,
};
use std::{marker::PhantomData, pin::Pin};
use uuid::Uuid;

use crate::{
    actors::{converter::incoming::Decode, engine::EngineActor},
    messages::internal::UserMessage,
};
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
    type Output = UserMessage;

    type Error = Infallible;

    fn decode(item: WsMessage) -> eyre::Result<Self::Output> {
        match item {
            WsMessage::Text(_) => unimplemented!(),
            WsMessage::Binary(bytes) => {
                let message: UserMessage =
                    serde_json::from_slice(bytes.as_ref())?;
                eyre::Ok(message)
            }
            WsMessage::Ping(bytes) => unimplemented!(),
            WsMessage::Pong(bytes) => unimplemented!(),
            WsMessage::Close(close_frame) => unimplemented!(),
        }
    }
}
///This is just a impl for testing the behaviour of the IncomingMessageActor
#[cfg(test)]
impl<I, S> Decode<UserMessage> for IncomingMessageActor<I, S>
where
    S: Stream<Item = I> + Send + 'static,
    I: Send + Sync + 'static,
{
    type Output = UserMessage;

    type Error = Infallible;

    fn decode(item: UserMessage) -> eyre::Result<Self::Output> {
        Ok(item)
    }
}

impl<I, S> Message<StreamMessage<I, (), ()>> for IncomingMessageActor<I, S>
where
    S: Stream<Item = I> + Send + 'static,
    I: Send + Sync + 'static,
    Self: Decode<I, Output = UserMessage> + Send,
{
    type Reply = ();

    async fn handle(
        &mut self,
        msg: StreamMessage<I, (), ()>,
        ctx: &mut kameo::prelude::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        match msg {
            StreamMessage::Next(msg) => {
                println!("inside incoming");
                let decoded = Self::decode(msg);
                if let std::result::Result::Ok(msg) = decoded {
                    let _ = self.engine.tell(msg).await;
                }
            }
            StreamMessage::Started(_) => println!("started"),
            //TODO: check to make sure implementation is secure
            StreamMessage::Finished(_) => {
                println!("finished");
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
