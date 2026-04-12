use std::{collections::HashMap, str::FromStr};

use crabby_specs::{
    grpc::groups::{
        ListGroupMembersRequest,
        group_service_client::{self, GroupServiceClient},
    },
    nats::{
        channel::{
            FanoutMessageDelivery, GroupChangeEvent, GroupChangeId,
            UserMessageDelivery,
        },
        subscriber::{FanoutStream, GroupEventStream},
        transport::NatsCoreTransport,
    },
    ws::outgoing::CrabbyWsFromServer,
};
use crabby_transport::{
    publisher::Publisher, subscriber::Subscriber, transport::Transport,
};
use eyre::Result;
use kameo::{
    Actor, actor::Spawn, error::Infallible, message::StreamMessage,
    prelude::Message,
};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    let gs_endpoint = std::env::var("GROUP_SERVICE_ENDPOINT")
        .unwrap_or("localhost:6969".to_string());
    let transport = NatsCoreTransport::new().await?;
    let gs_client =
        group_service_client::GroupServiceClient::connect(gs_endpoint).await?;

    let fanout =
        FanoutService::new(gs_client, HashMap::default(), transport.clone());
    let fanout_ref = FanoutService::spawn(fanout);

    let fanout_sub =
        Transport::<FanoutMessageDelivery>::subscriber(&transport)?;
    let user_message_stream: FanoutStream =
        Subscriber::<FanoutMessageDelivery>::subscribe(
            &fanout_sub,
            FanoutMessageDelivery,
        )
        .await?;
    let event_sub = Transport::<GroupChangeEvent>::subscriber(&transport)?;
    let group_event_stream: GroupEventStream =
        Subscriber::<GroupChangeEvent>::subscribe(&event_sub, GroupChangeEvent)
            .await?;

    fanout_ref.attach_stream(user_message_stream, (), ());
    fanout_ref.attach_stream(group_event_stream, (), ());

    tokio::signal::ctrl_c().await?;
    Ok(())
}

pub struct FanoutService {
    group_client: GroupServiceClient<tonic::transport::Channel>,
    cache: HashMap<Uuid, GroupState>,
    transport: NatsCoreTransport,
}
impl FanoutService {
    pub fn new(
        group_client: GroupServiceClient<tonic::transport::Channel>,
        cache: HashMap<Uuid, GroupState>,
        transport: NatsCoreTransport,
    ) -> Self {
        Self {
            group_client,
            cache,
            transport,
        }
    }
}

pub struct GroupState {
    group_version: u64,
    members: Vec<Uuid>,
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
                let CrabbyWsFromServer::ChatMessage {
                    ref dest,
                    ref user_id,
                    ..
                } = message;
                let recipients = match dest {
                    crabby_specs::ws::common::Destination::Individual {
                        id,
                    } => {
                        vec![*id]
                    }
                    crabby_specs::ws::common::Destination::Group { id } => {
                        self.cache
                            .get(id)
                            .map(|state| state.members.clone())
                            .unwrap_or_default()
                    }
                };
                for recipient_id in recipients {
                    if recipient_id.eq(user_id) {
                        continue;
                    }
                    let channel =
                        UserMessageDelivery::new(&recipient_id.to_string());
                    if let Ok(publisher) =
                        Transport::<UserMessageDelivery>::publisher(
                            &self.transport,
                            &channel,
                        )
                    {
                        let _ = Publisher::<UserMessageDelivery>::publish(
                            &publisher,
                            message.clone(),
                        )
                        .await;
                    }
                }
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
                let req = ListGroupMembersRequest {
                    group_id: event.0.to_string(),
                };
                if let Ok(resp) =
                    self.group_client.list_group_members(req).await
                {
                    let resp = resp.into_inner();
                    let ids = resp
                        .user_id
                        .into_iter()
                        //TODO: very poor error handling
                        .map(|id| Uuid::from_str(&id).unwrap_or_default())
                        .collect();
                    let state = GroupState {
                        group_version: resp.ver,
                        members: ids,
                    };
                    self.cache.insert(event.0, state);
                }
            }
            StreamMessage::Next(Err(_)) => {}
            StreamMessage::Started(_) => (),
            StreamMessage::Finished(_) => (),
        }
    }
}


