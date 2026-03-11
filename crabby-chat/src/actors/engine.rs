use crate::messages::internal::{UserConnected, UserDisconnected, UserMessage};
use crabby_specs::WsApi;
use hashbrown::HashMap;
use kameo::{
    Actor,
    actor::{ActorRef, Recipient},
    error::Infallible,
    prelude::Message,
};
use uuid::Uuid;

#[derive(Debug)]
pub struct EngineActor {
    map: HashMap<Uuid, Recipient<UserMessage>>,
}
impl Actor for EngineActor {
    type Args = Self;

    type Error = Infallible;

    async fn on_start(
        args: Self::Args,
        _actor_ref: ActorRef<Self>,
    ) -> Result<Self, Self::Error> {
        Ok(args)
    }
}
impl EngineActor {
    pub fn new(map: HashMap<Uuid, Recipient<UserMessage>>) -> Self {
        Self { map }
    }
}
impl Message<UserMessage> for EngineActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: UserMessage,
        ctx: &mut kameo::prelude::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        // if let Some(outgoing) = self.map.get(&msg.to) {
        //     let _ = outgoing.tell(msg).await;
        // }
        let recipients: Vec<_> = self.map.values().cloned().collect();

        for recipient in recipients {
            let _ = recipient.tell(msg.clone()).await;
        }
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
impl Message<UserConnected> for EngineActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: UserConnected,
        ctx: &mut kameo::prelude::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.map.insert(msg.0, msg.1);
    }
}
