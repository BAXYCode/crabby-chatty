use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use crabby_fanout::{
    service::FanoutService,
    traits::{GroupMembershipClient, UserMessagePublisher},
};
use crabby_specs::ws::{common::Destination, outgoing::CrabbyWsFromServer};
use eyre::Result;
use kameo::actor::Spawn;
use uuid::Uuid;

// --- Mock implementations ---

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

struct RecordingPublisher {
    published: Arc<Mutex<Vec<(Uuid, CrabbyWsFromServer)>>>,
}

impl RecordingPublisher {
    fn new() -> (Self, Arc<Mutex<Vec<(Uuid, CrabbyWsFromServer)>>>) {
        let published = Arc::new(Mutex::new(Vec::new()));
        (
            Self {
                published: published.clone(),
            },
            published,
        )
    }
}

#[async_trait]
impl UserMessagePublisher for RecordingPublisher {
    async fn publish_to_user(
        &self,
        recipient_id: Uuid,
        message: CrabbyWsFromServer,
    ) -> Result<()> {
        self.published
            .lock()
            .unwrap()
            .push((recipient_id, message));
        Ok(())
    }
}

fn make_chat_message(
    sender: Uuid,
    dest: Destination,
    contents: &str,
) -> CrabbyWsFromServer {
    CrabbyWsFromServer::ChatMessage {
        message_id: 1,
        user_id: sender,
        dest,
        timestamp: "2026-01-01T00:00:00Z".to_string(),
        contents: contents.to_string(),
    }
}

// --- Tests ---

#[tokio::test]
async fn individual_message_publishes_to_recipient() {
    let sender = Uuid::from_u128(1);
    let recipient = Uuid::from_u128(2);

    let (publisher, published) = RecordingPublisher::new();
    let group_client = MockGroupClient::new(vec![], 0);

    let fanout_ref = FanoutService::spawn(FanoutService::new(
        Box::new(group_client),
        Box::new(publisher),
    ));

    let chat_msg = make_chat_message(
        sender,
        Destination::Individual { id: recipient },
        "hello",
    );
    // Send via tell (fire-and-forget message)
    let stream_msg =
        kameo::message::StreamMessage::<Result<CrabbyWsFromServer>, (), ()>::Next(
            Ok(chat_msg),
        );
    fanout_ref.tell(stream_msg).await.unwrap();

    // Give the actor time to process
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let records = published.lock().unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].0, recipient);
}

#[tokio::test]
async fn individual_message_does_not_publish_back_to_sender() {
    let sender = Uuid::from_u128(1);

    let (publisher, published) = RecordingPublisher::new();
    let group_client = MockGroupClient::new(vec![], 0);

    let fanout_ref = FanoutService::spawn(FanoutService::new(
        Box::new(group_client),
        Box::new(publisher),
    ));

    // Send a message where dest == sender
    let msg = make_chat_message(
        sender,
        Destination::Individual { id: sender },
        "echo",
    );
    let stream_msg =
        kameo::message::StreamMessage::<Result<CrabbyWsFromServer>, (), ()>::Next(
            Ok(msg),
        );
    fanout_ref.tell(stream_msg).await.unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let records = published.lock().unwrap();
    assert_eq!(records.len(), 0, "sender should not receive their own message");
}

#[tokio::test]
async fn group_message_fans_out_to_all_members_except_sender() {
    let sender = Uuid::from_u128(1);
    let member_a = Uuid::from_u128(2);
    let member_b = Uuid::from_u128(3);
    let group_id = Uuid::from_u128(100);

    let (publisher, published) = RecordingPublisher::new();
    let group_client =
        MockGroupClient::new(vec![sender, member_a, member_b], 1);

    let fanout_ref = FanoutService::spawn(FanoutService::new(
        Box::new(group_client),
        Box::new(publisher),
    ));

    // First, trigger a group change so the cache gets populated
    let group_change =
        kameo::message::StreamMessage::<Result<crabby_specs::nats::channel::GroupChangeId>, (), ()>::Next(
            Ok(crabby_specs::nats::channel::GroupChangeId(group_id)),
        );
    fanout_ref.tell(group_change).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Now send a group message
    let msg = make_chat_message(
        sender,
        Destination::Group { id: group_id },
        "hi group",
    );
    let stream_msg =
        kameo::message::StreamMessage::<Result<CrabbyWsFromServer>, (), ()>::Next(
            Ok(msg),
        );
    fanout_ref.tell(stream_msg).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let records = published.lock().unwrap();
    assert_eq!(records.len(), 2, "should fan out to 2 members (excluding sender)");

    let recipient_ids: Vec<Uuid> = records.iter().map(|(id, _)| *id).collect();
    assert!(recipient_ids.contains(&member_a));
    assert!(recipient_ids.contains(&member_b));
    assert!(!recipient_ids.contains(&sender));
}

#[tokio::test]
async fn group_message_to_unknown_group_publishes_nothing() {
    let sender = Uuid::from_u128(1);
    let group_id = Uuid::from_u128(999);

    let (publisher, published) = RecordingPublisher::new();
    let group_client = MockGroupClient::new(vec![], 0);

    let fanout_ref = FanoutService::spawn(FanoutService::new(
        Box::new(group_client),
        Box::new(publisher),
    ));

    // Send group message without populating cache
    let msg = make_chat_message(
        sender,
        Destination::Group { id: group_id },
        "hello?",
    );
    let stream_msg =
        kameo::message::StreamMessage::<Result<CrabbyWsFromServer>, (), ()>::Next(
            Ok(msg),
        );
    fanout_ref.tell(stream_msg).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let records = published.lock().unwrap();
    assert_eq!(records.len(), 0, "unknown group should produce no publishes");
}

#[tokio::test]
async fn group_change_event_updates_cache() {
    let member_a = Uuid::from_u128(10);
    let member_b = Uuid::from_u128(20);
    let group_id = Uuid::from_u128(100);
    let sender = Uuid::from_u128(1);

    let (publisher, published) = RecordingPublisher::new();
    let group_client = MockGroupClient::new(vec![member_a, member_b], 5);

    let fanout_ref = FanoutService::spawn(FanoutService::new(
        Box::new(group_client),
        Box::new(publisher),
    ));

    // Trigger group change
    let group_change =
        kameo::message::StreamMessage::<Result<crabby_specs::nats::channel::GroupChangeId>, (), ()>::Next(
            Ok(crabby_specs::nats::channel::GroupChangeId(group_id)),
        );
    fanout_ref.tell(group_change).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Now send a group message to verify cache was populated
    let msg = make_chat_message(
        sender,
        Destination::Group { id: group_id },
        "test",
    );
    let stream_msg =
        kameo::message::StreamMessage::<Result<CrabbyWsFromServer>, (), ()>::Next(
            Ok(msg),
        );
    fanout_ref.tell(stream_msg).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let records = published.lock().unwrap();
    assert_eq!(records.len(), 2);
}
