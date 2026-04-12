//! End-to-end test: group chat with fanout delivery.
//!
//! Prerequisites — bring up the full stack first:
//!   docker compose -f docker/docker-compose.e2e.yml up -d --build
//!
//! Then run:
//!   cargo test -p e2e -- --ignored

use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use http::Request;
use serde::{Deserialize, Serialize};
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMsg};
use uuid::Uuid;

// --- Config (matches docker-compose.e2e.yml) ---

const TRAEFIK_URL: &str = "http://localhost:9099";
const WS_URL: &str = "ws://localhost:9099/ws";
const NATS_URL: &str = "nats://localhost:4222";

// --- Wire types (mirror crabby-specs) ---

#[derive(Serialize)]
#[serde(tag = "type")]
enum ClientMsg {
    UserMessage {
        user_id: Uuid,
        dest: Destination,
        timestamp: String,
        contents: String,
    },
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
enum ServerMsg {
    ChatMessage {
        message_id: u64,
        user_id: Uuid,
        dest: Destination,
        timestamp: String,
        contents: String,
    },
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
enum Destination {
    Individual { id: Uuid },
    Group { id: Uuid },
}

// --- Group service types ---

#[derive(Serialize)]
struct CreateGroupPayload {
    creator_id: Uuid,
    group_members: Vec<Uuid>,
}

// --- NATS group change event (msgpack, mirrors GroupChangeId) ---

#[derive(Serialize)]
struct GroupChangeId(Uuid);

// --- Helpers ---

async fn create_group(creator: Uuid, members: &[Uuid]) -> Uuid {
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{TRAEFIK_URL}/group"))
        .json(&CreateGroupPayload {
            creator_id: creator,
            group_members: members.to_vec(),
        })
        .send()
        .await
        .expect("POST /group failed");

    assert_eq!(
        resp.status().as_u16(),
        201,
        "create_group status: {}",
        resp.status()
    );

    resp.json::<Uuid>().await.expect("parse group_id")
}

async fn publish_group_change(group_id: Uuid) {
    let client = async_nats::connect(NATS_URL).await.expect("connect NATS");
    let payload = rmp_serde::to_vec(&GroupChangeId(group_id))
        .expect("encode GroupChangeId");
    client
        .publish("groups.events", payload.into())
        .await
        .expect("publish group change");
    client.flush().await.expect("flush");
}

type WsStream = tokio_tungstenite::WebSocketStream<
    tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
>;

async fn connect_ws(user_id: Uuid) -> WsStream {
    let req = Request::builder()
        .uri(WS_URL)
        .header("authorization", user_id.to_string())
        .header("host", "localhost:9090")
        .header("connection", "Upgrade")
        .header("upgrade", "websocket")
        .header("sec-websocket-version", "13")
        .header(
            "sec-websocket-key",
            tokio_tungstenite::tungstenite::handshake::client::generate_key(),
        )
        .body(())
        .expect("build ws request");

    let (ws, _) = connect_async(req).await.expect("ws connect");
    ws
}

async fn send_group_message(
    ws: &mut WsStream,
    sender: Uuid,
    group_id: Uuid,
    contents: &str,
) {
    let msg = ClientMsg::UserMessage {
        user_id: sender,
        dest: Destination::Group { id: group_id },
        timestamp: "2026-01-01T00:00:00Z".to_string(),
        contents: contents.to_string(),
    };
    let payload = serde_json::to_vec(&msg).expect("encode");
    ws.send(WsMsg::Binary(payload.into()))
        .await
        .expect("ws send");
}

async fn collect_messages(
    ws: &mut WsStream,
    expected: usize,
) -> Vec<ServerMsg> {
    let mut out = Vec::with_capacity(expected);
    let deadline = Duration::from_secs(10);

    for _ in 0..expected {
        let frame = tokio::time::timeout(deadline, ws.next())
            .await
            .expect("timed out waiting for message")
            .expect("stream ended")
            .expect("ws error");

        match frame {
            WsMsg::Binary(bytes) => {
                let msg: ServerMsg =
                    serde_json::from_slice(&bytes).expect("decode ServerMsg");
                out.push(msg);
            }
            other => panic!("unexpected frame: {other:?}"),
        }
    }
    out
}

async fn assert_no_messages(ws: &mut WsStream) {
    let result =
        tokio::time::timeout(Duration::from_millis(500), ws.next()).await;
    assert!(result.is_err(), "expected no messages but got one");
}

// --- Test ---

/// Full e2e: create a 5-member group, connect 3 users, send 10
/// messages, verify the two non-sender connected users each receive
/// all 10.
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn group_chat_fanout_e2e() {
    // 5 distinct users
    let users: Vec<Uuid> =
        (1..=5u128).map(|i| Uuid::from_u128(i * 10_000)).collect();
    let sender = users[0];

    // 1. Create the group via the group service HTTP API
    let group_id = create_group(sender, &users[1..]).await;

    // 2. Notify fanout to cache the membership
    publish_group_change(group_id).await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    // 3. Connect 3 users: sender + user2 + user3
    let mut ws_sender = connect_ws(sender).await;
    let mut ws_user2 = connect_ws(users[1]).await;
    let mut ws_user3 = connect_ws(users[2]).await;

    // Small settle time for engine to register connections
    tokio::time::sleep(Duration::from_millis(200)).await;

    // 4. Sender sends 10 messages to the group
    for i in 0..10 {
        send_group_message(
            &mut ws_sender,
            sender,
            group_id,
            &format!("msg-{i}"),
        )
        .await;
    }

    // 5. user2 and user3 should each receive all 10 messages
    let msgs_user2 = collect_messages(&mut ws_user2, 10).await;
    let msgs_user3 = collect_messages(&mut ws_user3, 10).await;

    assert_eq!(msgs_user2.len(), 10);
    assert_eq!(msgs_user3.len(), 10);

    // Verify contents arrived in order
    for (i, msg) in msgs_user2.iter().enumerate() {
        match msg {
            ServerMsg::ChatMessage {
                contents, user_id, ..
            } => {
                assert_eq!(contents, &format!("msg-{i}"));
                assert_eq!(*user_id, sender);
            }
        }
    }
    for (i, msg) in msgs_user3.iter().enumerate() {
        match msg {
            ServerMsg::ChatMessage {
                contents, user_id, ..
            } => {
                assert_eq!(contents, &format!("msg-{i}"));
                assert_eq!(*user_id, sender);
            }
        }
    }

    // 6. Sender should NOT receive their own messages (fanout excludes
    //    sender)
    assert_no_messages(&mut ws_sender).await;
}
