use crate::messages::Outgoing;
use futures::{Sink, SinkExt, Stream};
use kameo::{Actor, error::Infallible, prelude::Message};
use serde::Serialize;
use std::marker::PhantomData;
pub struct OutgoingMessageActor<S, M>
where
    S: SinkExt<M>,
{
    sink: S,
    _phantom: PhantomData<M>,
}

impl<S, M> Actor for OutgoingMessageActor<S, M>
where
    S: SinkExt<M> + Send + Sync + 'static + futures::Sink<M>,
    M: Send + Serialize + 'static,
{
    type Args = Self;

    type Error = Infallible;

    async fn on_start(
        args: Self::Args,
        actor_ref: kameo::prelude::ActorRef<Self>,
    ) -> Result<Self, Self::Error> {
        Ok(args)
    }
}

impl<S, M> Message<Outgoing> for OutgoingMessageActor<S, M>
where
    S: SinkExt<M> + Send + Sync + 'static + futures::Sink<M> + Unpin,
    M: Send + Serialize + 'static,
{
    type Reply = ();

    async fn handle(
        &mut self,
        msg: Outgoing,
        ctx: &mut kameo::prelude::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let res = self.sink.send(serde_json::to_vec(&msg).unwrap()).await;

        ()
    }
}
