use std::sync::Arc;

use async_trait::async_trait;
use crabby_chat::{
    actors::engine::EngineActor,
    id::{GenerateId, IdGenerator},
    messages::internal::{UserConnected, UserDisconnected},
};
use crabby_specs::{
    nats::{channel::FanoutMessageDelivery, delivery::PayloadWithDestination},
    ws::{
        common::Destination,
        incoming::CrabbyWsFromClient,
        outgoing::CrabbyWsFromServer,
    },
};
use crabby_transport::publisher::Publisher;
use eyre::Result;
use hashbrown::HashMap;
use kameo::{
    Actor,
    actor::{ActorRef, Spawn},
    error::Infallible,
    prelude::Message,
};
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

/// A fake publisher that captures everything published to it.
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

/// A minimal actor that collects CrabbyWsFromServer messages it receives.
/// We spawn one per "user" and register its Recipient with the engine.
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

fn make_engine(
    id: u64,
) -> (
    EngineActor,
    mpsc::UnboundedReceiver<CrabbyWsFromServer>,
) {
    let (pub_tx, pub_rx) = mpsc::unbounded_channel();
    let publisher = Arc::new(RecordingPublisher { tx: pub_tx });
    let id_gen = IdGenerator::new(FixedIdGenerator(id));
    let engine = EngineActor::new(HashMap::default(), id_gen, publisher);
    (engine, pub_rx)
}

async fn spawn_collector() -> (
    ActorRef<CollectorActor>,
    mpsc::UnboundedReceiver<CrabbyWsFromServer>,
) {
    let (tx, rx) = mpsc::unbounded_channel();
    let actor_ref = CollectorActor::spawn(CollectorActor { tx });
    (actor_ref, rx)
}

fn user_message(
    user_id: Uuid,
    dest: Uuid,
    contents: &str,
) -> CrabbyWsFromClient {
    CrabbyWsFromClient::UserMessage {
        user_id,
        dest: Destination::Individual { id: dest },
        timestamp: "2026-01-01T00:00:00Z".to_string(),
        contents: contents.to_string(),
    }
}

// -- Tests --

/// Sending CrabbyWsFromClient publishes to the fanout publisher
/// with a generated message_id.
#[tokio::test]
async fn incoming_message_publishes_to_fanout() {
    let (engine, mut pub_rx) = make_engine(42);
    let engine_ref = EngineActor::spawn(engine);

    let sender = Uuid::from_u128(1);
    let dest = Uuid::from_u128(2);
    let _ = engine_ref.tell(user_message(sender, dest, "hello")).await;

    let published = pub_rx.recv().await.expect("should have published");
    match published {
        CrabbyWsFromServer::ChatMessage {
            message_id,
            user_id,
            contents,
            ..
        } => {
            assert_eq!(message_id, 42);
            assert_eq!(user_id, sender);
            assert_eq!(contents, "hello");
        }
    }
}

/// UserConnected registers a recipient, UserDisconnected removes it.
#[tokio::test]
async fn connect_then_disconnect_user() {
    let (engine, _pub_rx) = make_engine(1);
    let engine_ref = EngineActor::spawn(engine);

    let user_id = Uuid::from_u128(10);
    let (collector_ref, mut collector_rx) = spawn_collector().await;

    // Connect user
    let _ = engine_ref
        .tell(UserConnected(user_id, collector_ref.recipient()))
        .await;

    // Deliver a message to that user
    let delivery = PayloadWithDestination {
        user_id,
        message: CrabbyWsFromServer::ChatMessage {
            message_id: 1,
            user_id: Uuid::from_u128(99),
            dest: Destination::Individual { id: user_id },
            timestamp: "t".into(),
            contents: "delivered".into(),
        },
    };
    let _ = engine_ref
        .tell(kameo::message::StreamMessage::<
            PayloadWithDestination,
            (),
            (),
        >::Next(delivery))
        .await;

    let msg = collector_rx.recv().await.expect("should receive delivery");
    match msg {
        CrabbyWsFromServer::ChatMessage { contents, .. } => {
            assert_eq!(contents, "delivered");
        }
    }

    // Disconnect
    let _ = engine_ref.tell(UserDisconnected(user_id)).await;

    // Further delivery to this user should not arrive
    let delivery2 = PayloadWithDestination {
        user_id,
        message: CrabbyWsFromServer::ChatMessage {
            message_id: 2,
            user_id: Uuid::from_u128(99),
            dest: Destination::Individual { id: user_id },
            timestamp: "t".into(),
            contents: "should not arrive".into(),
        },
    };
    let _ = engine_ref
        .tell(kameo::message::StreamMessage::<
            PayloadWithDestination,
            (),
            (),
        >::Next(delivery2))
        .await;

    // Give the actor a moment to process, then verify nothing arrived
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert!(
        collector_rx.try_recv().is_err(),
        "disconnected user should not receive messages"
    );
}

/// PayloadWithDestination delivered via StreamMessage routes to the
/// correct connected user.
#[tokio::test]
async fn delivery_routes_to_correct_user() {
    let (engine, _pub_rx) = make_engine(1);
    let engine_ref = EngineActor::spawn(engine);

    let alice = Uuid::from_u128(1);
    let bob = Uuid::from_u128(2);

    let (alice_ref, mut alice_rx) = spawn_collector().await;
    let (bob_ref, mut bob_rx) = spawn_collector().await;

    let _ = engine_ref
        .tell(UserConnected(alice, alice_ref.recipient()))
        .await;
    let _ = engine_ref
        .tell(UserConnected(bob, bob_ref.recipient()))
        .await;

    // Deliver a message targeted at alice only
    let delivery = PayloadWithDestination {
        user_id: alice,
        message: CrabbyWsFromServer::ChatMessage {
            message_id: 10,
            user_id: bob,
            dest: Destination::Individual { id: alice },
            timestamp: "t".into(),
            contents: "for alice".into(),
        },
    };
    let _ = engine_ref
        .tell(kameo::message::StreamMessage::<
            PayloadWithDestination,
            (),
            (),
        >::Next(delivery))
        .await;

    let msg = alice_rx.recv().await.expect("alice should receive");
    match msg {
        CrabbyWsFromServer::ChatMessage { contents, .. } => {
            assert_eq!(contents, "for alice");
        }
    }

    // Bob should not have received anything
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    assert!(
        bob_rx.try_recv().is_err(),
        "bob should not receive alice's message"
    );
}

/// Delivery for a user that isn't connected doesn't panic.
#[tokio::test]
async fn delivery_to_unknown_user_is_ignored() {
    let (engine, _pub_rx) = make_engine(1);
    let engine_ref = EngineActor::spawn(engine);

    let unknown = Uuid::from_u128(999);
    let delivery = PayloadWithDestination {
        user_id: unknown,
        message: CrabbyWsFromServer::ChatMessage {
            message_id: 1,
            user_id: Uuid::nil(),
            dest: Destination::Individual { id: unknown },
            timestamp: "t".into(),
            contents: "ghost".into(),
        },
    };
    // Should not panic
    let _ = engine_ref
        .tell(kameo::message::StreamMessage::<
            PayloadWithDestination,
            (),
            (),
        >::Next(delivery))
        .await;
}

/// Multiple messages publish with the same fixed ID (verifying the
/// id generator is called each time).
#[tokio::test]
async fn multiple_messages_all_get_ids() {
    let (engine, mut pub_rx) = make_engine(7);
    let engine_ref = EngineActor::spawn(engine);

    let user = Uuid::from_u128(1);
    let dest = Uuid::from_u128(2);

    for _ in 0..3 {
        let _ = engine_ref.tell(user_message(user, dest, "msg")).await;
    }

    for _ in 0..3 {
        let msg = pub_rx.recv().await.expect("should receive");
        match msg {
            CrabbyWsFromServer::ChatMessage { message_id, .. } => {
                assert_eq!(message_id, 7);
            }
        }
    }
}

/// Attach a stream of PayloadWithDestination and verify the engine
/// processes items from it.
#[tokio::test]
async fn attach_delivery_stream() {
    let (engine, _pub_rx) = make_engine(1);
    let engine_ref = EngineActor::spawn(engine);

    let user_id = Uuid::from_u128(42);
    let (collector_ref, mut collector_rx) = spawn_collector().await;

    let _ = engine_ref
        .tell(UserConnected(user_id, collector_ref.recipient()))
        .await;

    // Create a stream that yields two deliveries
    let deliveries = vec![
        PayloadWithDestination {
            user_id,
            message: CrabbyWsFromServer::ChatMessage {
                message_id: 1,
                user_id: Uuid::nil(),
                dest: Destination::Individual { id: user_id },
                timestamp: "t".into(),
                contents: "first".into(),
            },
        },
        PayloadWithDestination {
            user_id,
            message: CrabbyWsFromServer::ChatMessage {
                message_id: 2,
                user_id: Uuid::nil(),
                dest: Destination::Individual { id: user_id },
                timestamp: "t".into(),
                contents: "second".into(),
            },
        },
    ];
    let stream = futures_util::stream::iter(deliveries);
    engine_ref.attach_stream(Box::pin(stream), (), ());

    // Give the actor time to process the stream
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let msg1 = collector_rx.try_recv().expect("should receive first");
    let msg2 = collector_rx.try_recv().expect("should receive second");

    match (msg1, msg2) {
        (
            CrabbyWsFromServer::ChatMessage {
                contents: c1, ..
            },
            CrabbyWsFromServer::ChatMessage {
                contents: c2, ..
            },
        ) => {
            assert_eq!(c1, "first");
            assert_eq!(c2, "second");
        }
    }
}
