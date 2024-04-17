#![allow(dead_code, unused_variables, unused_imports)]
mod api;
mod event;
// use api::rest;
use axum::{
    extract::{
        ws::{Message, WebSocket},
        ConnectInfo, State, WebSocketUpgrade,
    },
    response::IntoResponse,
    routing::get,
};
use crabby_core::engine::Engine;
use event::ServerEvent;
use futures::{SinkExt, StreamExt};
use hashbrown::HashMap;
use std::net::SocketAddr;
use tokio::{
    net::TcpListener,
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
};
use uuid::Uuid;
#[tokio::main]
async fn main() {
    test().await;
}

async fn test() {
    let (sx, mut rx) = tokio::sync::mpsc::unbounded_channel::<ServerEvent>();
    let state = ChannelState { inner: sx };
    let listener = TcpListener::bind("0.0.0.0:6969").await.unwrap();
    let router = axum::Router::new()
        .route("/ws", get(websocket))
        .with_state(state);
    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}
async fn websocket(
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<ChannelState<ServerEvent>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| websocket_handler(socket, addr, state))
}

async fn websocket_handler(ws: WebSocket, addr: SocketAddr, state: ChannelState<ServerEvent>) {
    let (mut sink, mut stream) = ws.split();
    let (send, mut recv) = tokio::sync::mpsc::unbounded_channel::<Message>();
    let _receiving_task = tokio::task::spawn(async move {
        loop {
            let message = stream.next().await.unwrap().unwrap();
            println!("message received: {message:?}");
            let _ = send.send(message);
        }
    });
    let _sending_task = tokio::task::spawn(async move {
        loop {
            let message = recv.recv().await.unwrap();
            if let Message::Text(message) = message {
                let response = format!("here's your message: {message}");
                sink.send(Message::Text(response)).await.unwrap()
            }
        }
    });
}
struct ChatEngine {
    map: hashbrown::HashMap<Uuid, UnboundedSender<ServerEvent>>,
    rx: UnboundedReceiver<ServerEvent>,
}
impl ChatEngine {
    fn build(rx: UnboundedReceiver<ServerEvent>) -> Self {
        Self {
            map: HashMap::new(),
            rx,
        }
    }
}
impl Engine for ChatEngine {
    async fn run() {}
}
#[derive(Clone)]
struct ChannelState<T> {
    inner: UnboundedSender<T>,
}
