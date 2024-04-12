mod api;
use api::rest;
use axum::{
    extract::{
        ws::{Message, WebSocket},
        ConnectInfo, WebSocketUpgrade,
    },
    response::IntoResponse,
    routing::get,
};
use futures::{SinkExt, StreamExt};
use std::net::SocketAddr;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    test().await;
}

async fn test() {
    let listener = TcpListener::bind("0.0.0.0:6969").await.unwrap();
    let router = axum::Router::new().route("/ws", get(websocket));
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
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| websocket_handler(socket, addr))
}

async fn websocket_handler(ws: WebSocket, addr: SocketAddr) {
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
