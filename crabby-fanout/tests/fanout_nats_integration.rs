use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use crabby_fanout::{
    nats_publisher::NatsUserMessagePublisher,
    service::FanoutService,
    traits::GroupMembershipClient,
};
use crabby_specs::{
    nats::{
        channel::{FanoutMessageDelivery, GroupChangeEvent, UserMessageDelivery},
        subscriber::{FanoutStream, GroupEventStream},
        transport::NatsCoreTransport,
    },
    ws::{common::Destination, outgoing::CrabbyWsFromServer},
};
use crabby_transport::{
    publisher::Publisher, subscriber::Subscriber, transport::Transport,
};
use eyre::Result;
use kameo::actor::Spawn;
use uuid::Uuid;

// Use a real NATS transport but mock the gRPC group client

#[derive(Clone)]
struct MockGroupClient {
    members: Arc<Mutex<Vec<Uuid>>>,
    version: u64,
}

impl MockGroupClient {
    fn new(members: Vec<Uuid>, version: u64) -> Self {
        Self {
            members: Arc::new(Mutex::new(members)),
            version,
        }
    }
}

#[async_trait]
impl GroupMembershipClient for MockGroupClient {
    async fn list_group_members(
        &mut self,
        _group_id: Uuid,
    ) -> Result<(Vec<Uuid>, u64)> {
        let members = self.members.lock().unwrap().clone();
        Ok((members, self.version))
    }
}

async fn make_transport() -> NatsCoreTransport {
    // Requires NATS_CORE_URL env var (e.g. nats://localhost:4222)
    NatsCoreTransport::new()
        .await
        .expect("NATS must be running for integration tests")
}

#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn fanout_delivers_individual_message_via_nats() {
    let transport = make_transport().await;
    let sender = Uuid::from_u128(1);
    let recipient = Uuid::from_u128(2);

    // Subscribe to the recipient's delivery channel BEFORE starting fanout
    let user_sub = Transport::<UserMessageDelivery>::subscriber(&transport)
        .expect("subscriber");
    let delivery_channel = UserMessageDelivery::new(&recipient.to_string());
    let mut user_stream = Subscriber::<UserMessageDelivery>::subscribe(
        &user_sub,
        delivery_channel,
    )
    .await
    .expect("subscribe");

    // Start fanout service with real NATS publisher
    let group_client = MockGroupClient::new(vec![], 0);
    let publisher = NatsUserMessagePublisher::new(transport.clone());
    let fanout_ref = FanoutService::spawn(FanoutService::new(
        Box::new(group_client),
        Box::new(publisher),
    ));

    // Subscribe fanout to the fanout delivery channel
    let fanout_sub =
        Transport::<FanoutMessageDelivery>::subscriber(&transport)
            .expect("subscriber");
    let fanout_stream: FanoutStream =
        Subscriber::<FanoutMessageDelivery>::subscribe(
            &fanout_sub,
            FanoutMessageDelivery,
        )
        .await
        .expect("subscribe");
    fanout_ref.attach_stream(fanout_stream, (), ());

    // Publish a message to the fanout channel
    let fanout_channel = FanoutMessageDelivery;
    let fanout_pub =
        Transport::<FanoutMessageDelivery>::publisher(&transport, &fanout_channel)
            .expect("publisher");
    let msg = CrabbyWsFromServer::ChatMessage {
        message_id: 42,
        user_id: sender,
        dest: Destination::Individual { id: recipient },
        timestamp: "2026-01-01T00:00:00Z".to_string(),
        contents: "hello via nats".to_string(),
    };
    Publisher::<FanoutMessageDelivery>::publish(&fanout_pub, msg)
        .await
        .expect("publish");

    // Wait for the message to arrive on the recipient's delivery stream
    use futures_util::StreamExt;
    let delivered = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        user_stream.next(),
    )
    .await
    .expect("timeout waiting for delivery")
    .expect("stream ended");

    let delivered = delivered.expect("decode error");
    match delivered {
        CrabbyWsFromServer::ChatMessage { contents, .. } => {
            assert_eq!(contents, "hello via nats");
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn fanout_delivers_group_message_to_members_via_nats() {
    let transport = make_transport().await;
    let sender = Uuid::from_u128(101);
    let member_a = Uuid::from_u128(102);
    let member_b = Uuid::from_u128(103);
    let group_id = Uuid::from_u128(200);

    // Subscribe to both members' delivery channels
    let sub_a = Transport::<UserMessageDelivery>::subscriber(&transport).unwrap();
    let mut stream_a = Subscriber::<UserMessageDelivery>::subscribe(
        &sub_a,
        UserMessageDelivery::new(&member_a.to_string()),
    )
    .await
    .unwrap();

    let sub_b = Transport::<UserMessageDelivery>::subscriber(&transport).unwrap();
    let mut stream_b = Subscriber::<UserMessageDelivery>::subscribe(
        &sub_b,
        UserMessageDelivery::new(&member_b.to_string()),
    )
    .await
    .unwrap();

    // Start fanout with group members pre-configured
    let group_client =
        MockGroupClient::new(vec![sender, member_a, member_b], 1);
    let publisher = NatsUserMessagePublisher::new(transport.clone());
    let fanout_ref = FanoutService::spawn(FanoutService::new(
        Box::new(group_client),
        Box::new(publisher),
    ));

    // Attach both streams
    let fanout_sub =
        Transport::<FanoutMessageDelivery>::subscriber(&transport).unwrap();
    let fanout_stream: FanoutStream =
        Subscriber::<FanoutMessageDelivery>::subscribe(
            &fanout_sub,
            FanoutMessageDelivery,
        )
        .await
        .unwrap();
    fanout_ref.attach_stream(fanout_stream, (), ());

    let event_sub =
        Transport::<GroupChangeEvent>::subscriber(&transport).unwrap();
    let event_stream: GroupEventStream =
        Subscriber::<GroupChangeEvent>::subscribe(&event_sub, GroupChangeEvent)
            .await
            .unwrap();
    fanout_ref.attach_stream(event_stream, (), ());

    // Trigger group change to populate cache
    let event_pub = Transport::<GroupChangeEvent>::publisher(
        &transport,
        &GroupChangeEvent,
    )
    .unwrap();
    Publisher::<GroupChangeEvent>::publish(
        &event_pub,
        crabby_specs::nats::channel::GroupChangeId(group_id),
    )
    .await
    .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // Send group message
    let fanout_pub = Transport::<FanoutMessageDelivery>::publisher(
        &transport,
        &FanoutMessageDelivery,
    )
    .unwrap();
    let msg = CrabbyWsFromServer::ChatMessage {
        message_id: 99,
        user_id: sender,
        dest: Destination::Group { id: group_id },
        timestamp: "2026-01-01T00:00:00Z".to_string(),
        contents: "group hello".to_string(),
    };
    Publisher::<FanoutMessageDelivery>::publish(&fanout_pub, msg)
        .await
        .unwrap();

    use futures_util::StreamExt;
    let timeout = std::time::Duration::from_secs(5);

    let msg_a = tokio::time::timeout(timeout, stream_a.next())
        .await
        .expect("timeout for member_a")
        .expect("stream_a ended")
        .expect("decode error");

    let msg_b = tokio::time::timeout(timeout, stream_b.next())
        .await
        .expect("timeout for member_b")
        .expect("stream_b ended")
        .expect("decode error");

    match msg_a {
        CrabbyWsFromServer::ChatMessage { contents, .. } => {
            assert_eq!(contents, "group hello");
        }
    }
    match msg_b {
        CrabbyWsFromServer::ChatMessage { contents, .. } => {
            assert_eq!(contents, "group hello");
        }
    }
}
