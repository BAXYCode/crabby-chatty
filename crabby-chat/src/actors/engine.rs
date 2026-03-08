use hashbrown::HashMap;
use kameo::{Actor, actor::ActorRef, error::Infallible, prelude::Message};
use uuid::Uuid;

use crate::{
    actors::outgoing::OutgoingMessageActor,
    messages::{UserDisconnected, UserMessage},
};

#[derive(Debug)]
pub struct EngineActor {
    map: HashMap<Uuid, ()>,
}
impl Actor for EngineActor {
    type Args = Self;

    type Error = Infallible;

    async fn on_start(args: Self::Args, actor_ref: ActorRef<Self>) -> Result<Self, Self::Error> {
        unimplemented!()
    }
}

impl Message<UserMessage> for EngineActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: UserMessage,
        ctx: &mut kameo::prelude::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        todo!()
    }
}
impl Message<UserDisconnected> for EngineActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: UserDisconnected,
        ctx: &mut kameo::prelude::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        match msg {
            UserDisconnected(id) => self.map.remove(&id),
        };
    }
}

