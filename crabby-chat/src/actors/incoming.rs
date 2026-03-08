use axum::extract::ws::Message as WsMessage;
use eyre::Ok;
use futures::{Stream, stream::SplitStream};
use kameo::{Actor, actor::ActorRef, error::Infallible, message::StreamMessage, prelude::Message};
use std::marker::PhantomData;
use uuid::Uuid;

use crate::{
    actors::{converter::incoming::Decode, engine::EngineActor},
    messages::UserMessage,
};
pub type WsIncomingMessageActor<S> = IncomingMessageActor<WsMessage, S>;
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
                let message: UserMessage = serde_json::from_slice(bytes.as_ref())?;
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
                let decoded = Self::decode(msg);
                match decoded {
                    std::result::Result::Ok(msg) => {
                        self.engine.tell(msg).await;
                    }
                    _ => (),
                }
            }
            StreamMessage::Started(_) => todo!(),
            //TODO: check to make sure implementation is secure
            StreamMessage::Finished(_) => {
                if let Some(r) = self.me.take() {
                    if let Err(err) = r.stop_gracefully().await {
                        match err {
                            kameo::error::SendError::ActorNotRunning(_) => todo!(),
                            kameo::error::SendError::ActorStopped => todo!(),
                            kameo::error::SendError::MailboxFull(_) => todo!(),
                            kameo::error::SendError::HandlerError(_) => todo!(),
                            kameo::error::SendError::Timeout(_) => todo!(),
                        }
                    }
                } else {
                    drop(self);
                };
            }
        }
    }
}
