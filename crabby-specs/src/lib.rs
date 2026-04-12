use asyncapi_rust::AsyncApi;

use crate::ws::{incoming::CrabbyWsFromClient, outgoing::CrabbyWsFromServer};
pub mod grpc;
// #[cfg(feature = "nats")]
pub mod nats;
pub mod ws;
#[allow(clippy::duplicated_attributes)]
#[derive(AsyncApi)]
#[asyncapi(
    title = "Crabby-Chatty Chat Websocket API",
    version = "1.0",
    description = "Internal spec for communcation over websocket for \
                   Crabby-Chatty"
)]
#[asyncapi_channel(name = "chat", address = "ws")]
#[asyncapi_operation(name = "sendMessage", action = "send", channel = "chat", messages = [ CrabbyWsFromServer ])]
#[asyncapi_operation(
    name = "receiveMessage",
    action = "receive",
    channel = "chat",
    messages = [CrabbyWsFromClient]
)]
#[asyncapi_messages(CrabbyWsFromClient, CrabbyWsFromServer)]
pub struct WsApi;
