//! Integration tests for the engine actor with a real NATS server.
//! Start one with: docker compose -f docker/docker-compose.test.yml up -d
//!
//! These tests are ignored by default. Run with:
//!   cargo test -p crabby-chat --test nats_integration -- --ignored

use std::sync::Arc;

use async_trait::async_trait;
use crabby_chat::{
    actors::engine::EngineActor,
    id::{GenerateId, IdGenerator},
    messages::internal::{UserConnected, UserDisconnected},
};
use crabby_specs::{
    nats::{
        channel::FanoutMessageDelivery,
        delivery::user_delivery_stream,
    },
    ws::{
        common::Destination,
        incoming::CrabbyWsFromClient,
        outgoing::CrabbyWsFromServer,
    },
};
use crabby_transport::{codec::Codec, publisher::Publisher};
use eyre::Result;
use hashbrown::HashMap;
use kameo::{
    Actor,
    actor::{ActorRef, Spawn},
    error::Infallible,
    prelude::Message,
};
use futures_util::StreamExt;
use tokio::sync::mpsc;
use uuid::Uuid;

// -- Test helpers --

struct FixedIdGenerator(u64);

#[async_trait]
impl GenerateId for FixedIdGenerator {
    async fn id(&self) -> u64 {
        self.0
    }
}

/// Publisher that actually publishes to NATS (used as the engine's
/// fanout publisher so we can test the full outbound path).
struct NatsPublisher {
    client: async_nats::Client,
    subject: String,
}

#[async_trait]
impl Publisher<FanoutMessageDelivery> for NatsPublisher {
    async fn publish(&self, message: CrabbyWsFromServer) -> Result<()> {
        let payload =
            <crabby_transport::codec::JsonCodec as Codec<CrabbyWsFromServer>>::encode(
                &message,
            )?;
        self.client
            .publish(self.subject.clone(), payload)
            .await?;
        Ok(())
    }
}

/// Records all messages published to it (used when we don't need real NATS fanout).
struct RecordingPublisher {
    tx: mpsc::UnboundedSender<CrabbyWsFromServer>,
}

#[async_trait]
impl Publisher<FanoutMessageDelivery> for RecordingPublisher {
    async fn publish(&self, message: CrabbyWsFromServer) -> Result<()> {
        let _ = self.tx.send(message);
        Ok(())
    }
}

/// Minimal actor that collects CrabbyWsFromServer messages (stands
/// in for a real OutgoingWebsocketActor).
struct CollectorActor {
    tx: mpsc::UnboundedSender<CrabbyWsFromServer>,
}

impl Actor for CollectorActor {
    type Args = Self;
    type Error = Infallible;

    async fn on_start(
        args: Self::Args,
        _actor_ref: ActorRef<Self>,
    ) -> std::result::Result<Self, Self::Error> {
        Ok(args)
    }
}

impl Message<CrabbyWsFromServer> for CollectorActor {
    type Reply = ();

    async fn handle(
        &mut self,
        msg: CrabbyWsFromServer,
        _ctx: &mut kameo::prelude::Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let _ = self.tx.send(msg);
    }
}

async fn nats_client() -> async_nats::Client {
    let url =
        std::env::var("NATS_URL").unwrap_or_else(|_| "nats://localhost:4222".into());
    async_nats::connect(&url)
        .await
        .expect("failed to connect to NATS — is docker-compose.test.yml running?")
}

/// Helper to publish a CrabbyWsFromServer to a user delivery subject.
async fn publish_to_user(client: &async_nats::Client, user_id: Uuid, msg: &CrabbyWsFromServer) {
    let subject = format!("user.{}.delivery", user_id);
    let payload =
        <crabby_transport::codec::JsonCodec as Codec<CrabbyWsFromServer>>::encode(msg)
            .expect("encode failed");
    client.publish(subject, payload).await.expect("publish failed");
}

fn chat_message(id: u64, from: Uuid, to: Uuid, contents: &str) -> CrabbyWsFromServer {
    CrabbyWsFromServer::ChatMessage {
        message_id: id,
        user_id: from,
        dest: Destination::Individual { id: to },
        timestamp: "t".into(),
        contents: contents.into(),
    }
}

/// Spawn an engine with a NATS delivery stream attached and a
/// recording publisher for fanout.
async fn spawn_engine_with_nats(
    client: &async_nats::Client,
) -> (
    ActorRef<EngineActor>,
    mpsc::UnboundedReceiver<CrabbyWsFromServer>,
) {
    let (pub_tx, pub_rx) = mpsc::unbounded_channel();
    let publisher = Arc::new(RecordingPublisher { tx: pub_tx });
    let id_gen = IdGenerator::new(FixedIdGenerator(1));
    let engine = EngineActor::new(HashMap::default(), id_gen, publisher);
    let engine_ref = EngineActor::spawn(engine);

    let delivery_stream = user_delivery_stream(client.clone())
        .await
        .expect("subscribe failed");
    engine_ref.attach_stream(Box::pin(delivery_stream), (), ());

    // Let the subscription settle
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    (engine_ref, pub_rx)
}

async fn spawn_collector() -> (
    ActorRef<CollectorActor>,
    mpsc::UnboundedReceiver<CrabbyWsFromServer>,
) {
    let (tx, rx) = mpsc::unbounded_channel();
    let actor_ref = CollectorActor::spawn(CollectorActor { tx });
    (actor_ref, rx)
}

// -- Tests --

/// NATS publish -> delivery stream -> engine -> connected user's
/// recipient actor.
#[tokio::test]
#[ignore]
async fn engine_routes_nats_delivery_to_connected_user() {
    let client = nats_client().await;
    let (engine_ref, _pub_rx) = spawn_engine_with_nats(&client).await;

    let user_id = Uuid::from_u128(1);
    let (collector_ref, mut collector_rx) = spawn_collector().await;
    let _ = engine_ref
        .tell(UserConnected(user_id, collector_ref.recipient()))
        .await;

    publish_to_user(&client, user_id, &chat_message(55, Uuid::from_u128(2), user_id, "hello via nats")).await;

    let received = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        collector_rx.recv(),
    )
    .await
    .expect("timed out")
    .expect("channel closed");

    match received {
        CrabbyWsFromServer::ChatMessage {
            message_id,
            contents,
            ..
        } => {
            assert_eq!(message_id, 55);
            assert_eq!(contents, "hello via nats");
        }
    }
}

/// NATS delivery for a disconnected user is silently dropped — the
/// engine does not panic.
#[tokio::test]
#[ignore]
async fn engine_drops_nats_delivery_for_disconnected_user() {
    let client = nats_client().await;
    let (engine_ref, _pub_rx) = spawn_engine_with_nats(&client).await;

    let ghost = Uuid::from_u128(999);
    publish_to_user(&client, ghost, &chat_message(1, Uuid::nil(), ghost, "nobody home")).await;

    // Give the engine time to process — should not panic
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    // Engine should still be alive and accept messages
    let (collector_ref, _) = spawn_collector().await;
    let result = engine_ref
        .tell(UserConnected(ghost, collector_ref.recipient()))
        .await;
    assert!(result.is_ok(), "engine should still be running");
}

/// Multiple NATS deliveries for different users are routed to the
/// correct recipients.
#[tokio::test]
#[ignore]
async fn engine_routes_to_multiple_users_via_nats() {
    let client = nats_client().await;
    let (engine_ref, _pub_rx) = spawn_engine_with_nats(&client).await;

    let alice = Uuid::from_u128(10);
    let bob = Uuid::from_u128(20);

    let (alice_ref, mut alice_rx) = spawn_collector().await;
    let (bob_ref, mut bob_rx) = spawn_collector().await;

    let _ = engine_ref
        .tell(UserConnected(alice, alice_ref.recipient()))
        .await;
    let _ = engine_ref
        .tell(UserConnected(bob, bob_ref.recipient()))
        .await;

    publish_to_user(&client, alice, &chat_message(1, bob, alice, "hi alice")).await;
    publish_to_user(&client, bob, &chat_message(2, alice, bob, "hi bob")).await;

    let alice_msg = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        alice_rx.recv(),
    )
    .await
    .expect("timed out")
    .expect("alice channel closed");

    let bob_msg = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        bob_rx.recv(),
    )
    .await
    .expect("timed out")
    .expect("bob channel closed");

    match alice_msg {
        CrabbyWsFromServer::ChatMessage { contents, .. } => {
            assert_eq!(contents, "hi alice");
        }
    }
    match bob_msg {
        CrabbyWsFromServer::ChatMessage { contents, .. } => {
            assert_eq!(contents, "hi bob");
        }
    }
}

/// User connects, receives a NATS delivery, disconnects, and a
/// subsequent delivery is dropped.
#[tokio::test]
#[ignore]
async fn engine_stops_delivering_after_disconnect_via_nats() {
    let client = nats_client().await;
    let (engine_ref, _pub_rx) = spawn_engine_with_nats(&client).await;

    let user_id = Uuid::from_u128(42);
    let (collector_ref, mut collector_rx) = spawn_collector().await;
    let _ = engine_ref
        .tell(UserConnected(user_id, collector_ref.recipient()))
        .await;

    // First message should arrive
    publish_to_user(&client, user_id, &chat_message(1, Uuid::nil(), user_id, "first")).await;

    let msg = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        collector_rx.recv(),
    )
    .await
    .expect("timed out")
    .expect("channel closed");

    match msg {
        CrabbyWsFromServer::ChatMessage { contents, .. } => {
            assert_eq!(contents, "first");
        }
    }

    // Disconnect the user
    let _ = engine_ref.tell(UserDisconnected(user_id)).await;

    // Second message should not arrive
    publish_to_user(&client, user_id, &chat_message(2, Uuid::nil(), user_id, "after disconnect")).await;

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    assert!(
        collector_rx.try_recv().is_err(),
        "disconnected user should not receive messages"
    );
}

/// Engine publishes incoming client messages to the fanout publisher,
/// and those can be received via NATS subscription.
#[tokio::test]
#[ignore]
async fn engine_publishes_to_nats_fanout() {
    let client = nats_client().await;

    // Use a real NATS publisher for fanout
    let publisher = Arc::new(NatsPublisher {
        client: client.clone(),
        subject: "messages.delivery.fanout".into(),
    });
    let id_gen = IdGenerator::new(FixedIdGenerator(77));
    let engine = EngineActor::new(HashMap::default(), id_gen, publisher);
    let engine_ref = EngineActor::spawn(engine);

    // Subscribe to the fanout subject
    let mut sub = client
        .subscribe("messages.delivery.fanout")
        .await
        .expect("subscribe failed");

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Send a client message to the engine
    let _ = engine_ref
        .tell(CrabbyWsFromClient::UserMessage {
            user_id: Uuid::from_u128(1),
            dest: Destination::Individual {
                id: Uuid::from_u128(2),
            },
            timestamp: "t".into(),
            contents: "fanout test".into(),
        })
        .await;

    // Should receive the published message on the NATS subscription
    let nats_msg = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        sub.next(),
    )
    .await
    .expect("timed out")
    .expect("sub ended");

    let decoded: CrabbyWsFromServer =
        <crabby_transport::codec::JsonCodec as Codec<CrabbyWsFromServer>>::decode(
            &nats_msg.payload,
        )
        .expect("decode failed");

    match decoded {
        CrabbyWsFromServer::ChatMessage {
            message_id,
            contents,
            ..
        } => {
            assert_eq!(message_id, 77);
            assert_eq!(contents, "fanout test");
        }
    }
}
