use bytes::{Bytes, BytesMut};
use eyre::{Ok, Result};
use serde::{Serialize, de::DeserializeOwned};

///Trait to put bounds on how a message is encoded and decoded
pub trait Codec<M> {
    fn encode(message: &M) -> Result<Bytes>;
    fn decode(bytes: &[u8]) -> Result<M>;
}

pub struct JsonCodec;

impl<M> Codec<M> for JsonCodec
where
    M: Serialize + DeserializeOwned + 'static,
{
    fn encode(message: &M) -> Result<Bytes> {
        let bytes = Bytes::from(serde_json::to_vec_pretty(message)?);
        Ok(bytes)
    }

    fn decode(bytes: &[u8]) -> Result<M> {
        let res = serde_json::from_slice(bytes)?;
        Ok(res)
    }
}

pub struct MsgpackCodec;

impl<M> Codec<M> for MsgpackCodec
where
    M: Serialize + DeserializeOwned + 'static,
{
    fn encode(message: &M) -> Result<Bytes> {
        let bytes = Bytes::from(rmp_serde::to_vec(message)?);
        Ok(bytes)
    }

    fn decode(bytes: &[u8]) -> Result<M> {
        Ok(rmp_serde::from_slice(bytes)?)
    }
}
