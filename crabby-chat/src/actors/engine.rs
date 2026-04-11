use std::sync::Arc;

use crabby_specs::{
    nats::{channel::FanoutMessageDelivery, delivery::PayloadWithDestination},
    ws::{incoming::CrabbyWsFromClient, outgoing::CrabbyWsFromServer},
};
use crabby_transport::publisher::Publisher;
use hashbrown::HashMap;
use kameo::{
    Actor,
    actor::{ActorRef, Recipient},
    error::Infallible,
    message::StreamMessage,
    prelude::Message,
};
use tracing::warn;
use uuid::Uuid;

use crate::{
    id::{GenerateId, IdGenerator},
    messages::internal::{UserConnected, UserDisconnected},
};

pub struct EngineActor {
    map: HashMap<Uuid, Recipient<CrabbyWsFromServer>>,
    id_gen: IdGenerator,
    fanout: Arc<dyn Publisher<FanoutMessageDelivery>>,
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
        fanout: Arc<dyn Publisher<FanoutMessageDelivery>>,
    ) -> EngineActor {
        Self {
            map,
            id_gen,
            fanout,
        }
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
            } => {
                CrabbyWsFromServer::ChatMessage {
                    message_id: id,
                    user_id,
                    dest,
                    timestamp,
                    contents,
                }
            }
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
        // let recipients: Vec<_> = self.map.values().collect();
        let msg_with_id = self.make_outbound_message(msg).await;
        let _ = self.fanout.publish(msg_with_id).await;
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
impl Message<StreamMessage<PayloadWithDestination, (), ()>> for EngineActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: StreamMessage<PayloadWithDestination, (), ()>,
        _ctx: &mut kameo::prelude::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        match msg {
            StreamMessage::Next(delivery) => {
                if let Some(recipient) = self.map.get(&delivery.user_id) {
                    let _ = recipient.tell(delivery.message).await;
                } else {
                    warn!(
                        user_id = %delivery.user_id,
                        "received delivery for disconnected user"
                    );
                }
            }
            StreamMessage::Started(_) => (),
            StreamMessage::Finished(_) => {
                warn!("NATS delivery stream ended");
            }
        }
    }
}
