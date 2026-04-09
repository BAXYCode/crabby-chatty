pub struct JsonCodec;

impl<M> Codec<M> for JsonCodec
where
    M: Serialize + DeserializeOwned + 'static,
{
    fn encode(&self, message: &M) -> Result<Bytes> {
        let bytes = Bytes::from(serde_json::to_vec_pretty(message)?);
        Ok(bytes)
    }

    fn decode(bytes: &[u8]) -> Result<M> {
        let res = serde_json::from_slice(bytes)?;
        Ok(res)
    }
}
