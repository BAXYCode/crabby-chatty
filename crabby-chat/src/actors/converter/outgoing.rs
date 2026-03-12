use axum::{body::Bytes, extract::ws::Message};
use crabby_specs::ws::outgoing::CrabbyWsFromServer;
use eyre::Result;

pub trait Encode<I> {
    type Output;
    fn encode(item: I) -> Result<Self::Output>;
}
pub struct ServerToTransport;

impl Encode<CrabbyWsFromServer> for ServerToTransport {
    type Output = Message;

    fn encode(item: CrabbyWsFromServer) -> Result<Self::Output> {
        let serialized = serde_json::to_vec(&item).unwrap();
        let bytes =
            Message::Binary(Bytes::copy_from_slice(serialized.as_slice()));
        Ok(bytes)
    }
}
