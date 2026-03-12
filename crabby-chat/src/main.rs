#![allow(dead_code)]
mod actors;
mod client;
mod error;
mod handle;
pub mod id;
pub mod messages;
use axum::{
    extract::{ConnectInfo, FromRef, State, WebSocketUpgrade, ws::WebSocket},
    response::IntoResponse,
    routing::get,
};
use crabby_core::shutdown::shutdown_signal;
use ferroid::{generator::AtomicSnowflakeGenerator, time::MonotonicClock};
use futures::StreamExt;
use hashbrown::HashMap;
use kameo::actor::{ActorRef, Spawn};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing::{info, instrument};

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use uuid::{NoContext, Timestamp, Uuid};

use crate::{
    actors::{
        engine::EngineActor,
        incoming::{IncomingMessageActor, IncomingWebsocketActor},
        outgoing::OutgoingWebsocketActor,
    },
    id::IdGenerator,
};

#[tokio::main]
async fn main() {
    test().await;
}

#[instrument]
async fn test() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::filter::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                // axum logs rejections from built-in extractors with the `axum::rejection`
                // target, at `TRACE` level. `axum::rejection=trace` enables showing those events
                "example_tracing_aka_logging=debug,tower_http=debug,axum::rejection=trace".into()
            }),
        )
        .with(tracing_subscriber::fmt::layer().pretty())
        .init();
    let id_gen = IdGenerator::new(AtomicSnowflakeGenerator::new(
        0,
        MonotonicClock::default(),
    ));
    let engine = EngineActor::new(HashMap::default(), id_gen);
    //spawn Engine
    let engine_ref = EngineActor::spawn(engine);
    let state = SharedState {
        channel: ChannelState { inner: engine_ref },
    };
    let listener = TcpListener::bind("0.0.0.0:6969").await.unwrap();
    let router = axum::Router::new()
        .route("/ws", get(websocket))
        .with_state(state);
    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .unwrap();
}
async fn websocket(
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<ChannelState>,
) -> impl IntoResponse {
    info!("received connection from {addr}");
    ws.on_upgrade(move |socket| websocket_handler(socket, addr, state))
}

// TODO: add token extractor for extracting user UUID
async fn websocket_handler(
    ws: WebSocket,
    addr: SocketAddr,
    mut state: ChannelState,
) {
    let (mut sink, mut stream) = ws.split();
    //Later we extract will user id using some kind of interceptor
    let id = crate::id();
    let outbox =
        OutgoingWebsocketActor::new(sink, state.inner.clone(), id.clone());
    OutgoingWebsocketActor::spawn(outbox);
    let inbox: IncomingWebsocketActor =
        IncomingMessageActor::new(state.inner.clone(), id.clone());
    let stream = Box::pin(stream.filter_map(|item| async move { item.ok() }));
    let inbox_ref = IncomingWebsocketActor::spawn(inbox);
    inbox_ref.attach_stream(stream, (), ());
}

#[derive(Debug, Clone)]
struct ChannelState {
    inner: ActorRef<EngineActor>,
}
#[derive(Clone, Debug)]
struct SharedState {
    channel: ChannelState,
}
impl FromRef<SharedState> for ChannelState {
    fn from_ref(input: &SharedState) -> Self {
        input.channel.clone()
    }
}

fn id() -> Uuid {
    let ts = Timestamp::now(NoContext);
    Uuid::new_v7(ts)
}
