#![allow(unused_variables, unused_mut, unused_imports)]

use futures::{
    stream::{SplitSink, SplitStream, StreamExt},
    SinkExt,
};

use reqwest::Client;
use reqwest_websocket::{self, Message, RequestBuilderExt, WebSocket};
use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncBufReadExt, BufReader, Stdin},
    select,
};
use tokio_util::codec::length_delimited;
use tracing::info;
use uuid::{NoContext, Timestamp, Uuid};
#[tokio::main]
async fn main() {
    let io = tokio::io::stdin();
    let mut reader = BufReader::new(io);
    let connection = Client::default()
        .get("http://127.0.0.1:6969/ws")
        .upgrade()
        .send()
        .await
        .expect("Could not connect to remote");
    let websocket = connection
        .into_websocket()
        .await
        .expect("could not upgrade websocket");
    let (mut sink, mut stream) = websocket.split();

    let _send = tokio::spawn(async move { incoming_messages(stream).await });
    let _recv = tokio::spawn(async move { outgoing_message(sink, reader).await });

    select! {
        _ = _send =>return,
        _ = _recv => return
    }
}

async fn incoming_messages(mut income: SplitStream<WebSocket>) {
    info!("inside incoming");
    while let Some(serialized) = income.next().await {
        if let Ok(message) = serialized {
            let message = match message {
                Message::Text(str) => Vec::from(str),
                Message::Binary(bin) => bin,
            };
            let message: ChatMessage =
                serde_json::from_slice(&message).expect("Could not parse websocket message");
            println!("from engine: {:?}", message)
        }
    }
    println!("leaving incoming");
}

async fn outgoing_message(mut sink: SplitSink<WebSocket, Message>, mut reader: BufReader<Stdin>) {
    let mut buf = String::new();
    while let Ok(read) = reader.read_line(&mut buf).await {
        if read == 0 {
            return;
        }
        let blah = sink
            .send(Message::Binary(
                serde_json::to_vec(&ChatMessage::from_string(buf)).unwrap(),
            ))
            .await;
        buf = String::new();
    }
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    content: MessageContent,
    // id: Uuid,
    from: Uuid,
    // destination: Uuid,
}
#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) enum MessageContent {
    String(String),
    File(Vec<u8>),
    Photo(Vec<u8>),
}
fn id() -> Uuid {
    let ts = Timestamp::now(NoContext);
    Uuid::new_v7(ts)
}
impl ChatMessage {
    fn from_string(string: String) -> Self {
        ChatMessage {
            content: MessageContent::String(string),
            from: id(),
        }
    }
}
