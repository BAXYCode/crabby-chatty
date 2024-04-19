#![allow(dead_code, unused_variables, unused_imports, unused_mut)]
mod api;
mod error;
mod event;
mod event_types;
mod handle;
mod messages;
use api::rest;
use axum::{
    extract::{
        ws::{Message, WebSocket},
        ConnectInfo, FromRef, State, WebSocketUpgrade,
    },
    response::IntoResponse,
    routing::get,
};
use axum_macros::FromRef;
use crabby_core::{engine, shutdown};
use crabby_core::{engine::Engine, shutdown::shutdown_signal};
use error::ChatError;
use event::ServerEvent;
use event_types::connect;
use futures::{stream::SplitSink, stream::SplitStream, stream::Stream, SinkExt, StreamExt};
use handle::Handler;
use hashbrown::HashMap;
use messages::Message as ChatMessage;
use std::net::SocketAddr;
use tokio::{
    net::TcpListener,
    sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
};
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};
use tracing::{instrument, warn};
use uuid::{NoContext, Timestamp, Uuid};
#[tokio::main]
async fn main() {
    test().await;
}

#[instrument]
async fn test() {
    let (sx, rx) = tokio::sync::mpsc::unbounded_channel::<ServerEvent>();
    let engine = ChatEngine::build(rx);
    let state = SharedState {
        channel: ChannelState { inner: sx },
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
    ws.on_upgrade(move |socket| websocket_handler(socket, addr, state))
}
// TODO: add token extractor for extracting user UUID
#[instrument]
async fn websocket_handler(ws: WebSocket, addr: SocketAddr, mut state: ChannelState) {
    let (mut sink, mut stream) = ws.split();

    let mut recv = register(&mut state, None);
    let _recv = tokio::spawn(async move { incoming_handler(stream).await });
}
async fn incoming_handler(mut income: SplitStream<WebSocket>) {
    loop {
        let next = income.next().await;
        println!("{:?}", next)
    }
}
struct ChatEngine {
    map: hashbrown::HashMap<Uuid, UnboundedSender<ChatMessage>>,
    rx: UnboundedReceiver<ServerEvent>,
    db: hashbrown::HashMap<Uuid, User>,
}
impl ChatEngine {
    fn build(rx: UnboundedReceiver<ServerEvent>) -> Self {
        Self {
            map: hashbrown::HashMap::new(),
            rx,
            db: hashbrown::HashMap::new(),
        }
    }
    fn if_error(res: Result<(), ChatError>) {
        if let Some(err) = res.err() {
            warn!("{:?}", err);
        }
    }
}
impl Engine for ChatEngine {
    async fn run(&mut self) {
        loop {
            if let Some(event) = self.rx.recv().await {
                match event {
                    ServerEvent::Connected(connected) => {
                        let handled = self.handle(connected).await;
                        Self::if_error(handled);
                    }
                    ServerEvent::Disconnected(disconnect) => {
                        Self::if_error(self.handle(disconnect).await);
                    }
                    ServerEvent::ChatMessage(message) => {
                        Self::if_error(self.handle(message).await);
                    }
                }
            }
        }
    }
}
#[derive(Debug, Clone)]
struct User {
    name: String,
    email: String,
    password: String,
}
#[derive(Debug, Clone)]
struct ChannelState {
    inner: UnboundedSender<ServerEvent>,
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
// Function to inform the engine of a new user being connected
#[instrument]
fn register(
    state: &mut ChannelState,
    id: Option<Uuid>,
) -> Result<(UnboundedReceiver<ChatMessage>, Uuid), ChatError> {
    // channels for communication between the thread handling client connection and engine and vice versa
    let (mut sender, recv) = unbounded_channel::<ChatMessage>();
    let id = if let Some(id) = id { id } else { crate::id() };

    let connected = event_types::connect::Connect::new(sender, id);
    let connect = ServerEvent::Connected(connected);
    // Send notification to Engine that new user is connected, this is his Id and channel to send outgoing messages
    state.inner.send(connect)?;
    Ok((recv, id))
}
