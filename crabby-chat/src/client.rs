#![allow(unused_variables, unused_mut, unused_imports)]
use crabby_specs::WsApi;
use crabby_specs::ws::{
    incoming::CrabbyWsFromClient, outgoing::CrabbyWsFromServer,
};
use futures::{
    SinkExt,
    stream::{SplitSink, SplitStream, StreamExt},
};
use jiff::Timestamp;
use reqwest::Client;
use reqwest_websocket::{self, Message, WebSocket};
use reqwest_websocket::{Bytes, Upgrade};
use serde::{Deserialize, Serialize};
use tokio::{
    io::{AsyncBufReadExt, BufReader, Stdin},
    select,
};
use tracing::info;
use uuid::{NoContext, Timestamp as UuidTimestamp, Uuid};
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
    let id = id();
    let _recv =
        tokio::spawn(async move { outgoing_message(sink, reader, &id).await });

    select! {
        _ = _send =>return,
        _ = _recv => return
    }
}

async fn incoming_messages(mut income: SplitStream<WebSocket>) {
    while let Some(serialized) = income.next().await {
        if let Ok(message) = serialized {
            let message = match message {
                Message::Text(str) => Vec::from(str),
                Message::Binary(bin) => bin.to_vec(),
                Message::Close { .. } => {
                    break;
                }
                _ => {
                    continue;
                }
            };
            let message: CrabbyWsFromServer = serde_json::from_slice(&message)
                .expect("Could not parse websocket message");
            println!("{:?}", message);
        }
    }
}

async fn outgoing_message(
    mut sink: SplitSink<WebSocket, Message>,
    mut reader: BufReader<Stdin>,
    user_id: &Uuid,
) {
    let mut buf = String::new();
    while let Ok(read) = reader.read_line(&mut buf).await {
        if read == 0 {
            return;
        }
        println!("inside outgoing");
        let message = message_from_str(user_id, buf);
        let serialized = serde_json::to_vec_pretty(&message).unwrap();
        let bytes = Bytes::from(serialized);
        let blah = sink.send(Message::Binary(bytes)).await;
        buf = String::new();
    }
}

fn message_from_str(user_id: &Uuid, message: String) -> CrabbyWsFromClient {
    println!("{:?}", message);
    CrabbyWsFromClient::UserMessage {
        user_id: *user_id,
        dest: crabby_specs::ws::common::Destination::Individual { id: id() },
        timestamp: Timestamp::now().to_string(),
        contents: message,
    }
}

fn id() -> Uuid {
    let ts = UuidTimestamp::now(NoContext);
    Uuid::new_v7(ts)
}
