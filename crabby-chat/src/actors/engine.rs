use crate::{
    id::{GenerateId, IdGenerator},
    messages::internal::{UserConnected, UserDisconnected},
};
use crabby_specs::ws::{
    incoming::CrabbyWsFromClient, outgoing::CrabbyWsFromServer,
};
use hashbrown::HashMap;
use kameo::{
    Actor,
    actor::{ActorRef, Recipient},
    error::Infallible,
    prelude::Message,
};
use uuid::Uuid;

pub struct EngineActor {
    map: HashMap<Uuid, Recipient<CrabbyWsFromServer>>,
    id_gen: IdGenerator,
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
    pub fn new(
        map: HashMap<Uuid, Recipient<CrabbyWsFromServer>>,
        id_gen: IdGenerator,
    ) -> EngineActor {
        Self { map, id_gen }
    }
    async fn make_outbound_message(
        &self,
        message: CrabbyWsFromClient,
    ) -> CrabbyWsFromServer {
        let id = self.id_gen.id().await;
        match message {
            CrabbyWsFromClient::UserMessage {
                user_id,
                dest,
                timestamp,
                contents,
            } => CrabbyWsFromServer::ChatMessage {
                message_id: id,
                user_id,
                dest,
                timestamp,
                contents,
            },
        }
    }
}
impl Message<CrabbyWsFromClient> for EngineActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: CrabbyWsFromClient,
        _ctx: &mut kameo::prelude::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        // if let Some(outgoing) = self.map.get(&msg.to) {
        //     let _ = outgoing.tell(msg).await;
        // }
        let recipients: Vec<_> = self.map.values().cloned().collect();
        let msg_with_id = self.make_outbound_message(msg).await;
        for recipient in recipients {
            let _ = recipient.tell(msg_with_id.clone()).await;
        }
    }
}
impl Message<UserDisconnected> for EngineActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: UserDisconnected,
        _ctx: &mut kameo::prelude::Context<Self, Self::Reply>,
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
        _ctx: &mut kameo::prelude::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.map.insert(msg.0, msg.1);
    }
}

