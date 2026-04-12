use std::collections::HashMap;

use crabby_specs::{
    nats::channel::GroupChangeId,
    ws::{common::Destination, outgoing::CrabbyWsFromServer},
};
use eyre::Result;
use kameo::{
    Actor, error::Infallible, message::StreamMessage, prelude::Message,
};
use uuid::Uuid;

use crate::traits::{GroupMembershipClient, UserMessagePublisher};

pub struct FanoutService {
    group_client: Box<dyn GroupMembershipClient>,
    publisher: Box<dyn UserMessagePublisher>,
    cache: HashMap<Uuid, GroupState>,
}

pub struct GroupState {
    pub group_version: u64,
    pub members: Vec<Uuid>,
}

impl FanoutService {
    pub fn new(
        group_client: Box<dyn GroupMembershipClient>,
        publisher: Box<dyn UserMessagePublisher>,
    ) -> Self {
        Self {
            group_client,
            publisher,
            cache: HashMap::new(),
        }
    }

    async fn handle_chat_message(
        &self,
        message: CrabbyWsFromServer,
    ) {
        let CrabbyWsFromServer::ChatMessage {
            ref dest,
            ref user_id,
            ..
        } = message;
        let recipients = match dest {
            Destination::Individual { id } => vec![*id],
            Destination::Group { id } => self
                .cache
                .get(id)
                .map(|state| state.members.clone())
                .unwrap_or_default(),
        };
        for recipient_id in recipients {
            if recipient_id.eq(user_id) {
                continue;
            }
            let _ = self
                .publisher
                .publish_to_user(recipient_id, message.clone())
                .await;
        }
    }

    async fn handle_group_change(&mut self, group_id: Uuid) {
        if let Ok((members, version)) =
            self.group_client.list_group_members(group_id).await
        {
            let state = GroupState {
                group_version: version,
                members,
            };
            self.cache.insert(group_id, state);
        }
    }
}

impl Actor for FanoutService {
    type Args = Self;
    type Error = Infallible;

    async fn on_start(
        args: Self::Args,
        _actor_ref: kameo::prelude::ActorRef<Self>,
    ) -> Result<Self, Self::Error> {
        Ok(args)
    }
}

impl Message<StreamMessage<Result<CrabbyWsFromServer>, (), ()>>
    for FanoutService
{
    type Reply = ();

    async fn handle(
        &mut self,
        msg: StreamMessage<Result<CrabbyWsFromServer>, (), ()>,
        _ctx: &mut kameo::prelude::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        match msg {
            StreamMessage::Next(Ok(message)) => {
                self.handle_chat_message(message).await;
            }
            StreamMessage::Next(Err(_)) => {}
            StreamMessage::Started(_) => (),
            StreamMessage::Finished(_) => (),
        }
    }
}

impl Message<StreamMessage<Result<GroupChangeId>, (), ()>> for FanoutService {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: StreamMessage<Result<GroupChangeId>, (), ()>,
        _ctx: &mut kameo::prelude::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        match msg {
            StreamMessage::Next(Ok(event)) => {
                self.handle_group_change(event.0).await;
            }
            StreamMessage::Next(Err(_)) => {}
            StreamMessage::Started(_) => (),
            StreamMessage::Finished(_) => (),
        }
    }
}
