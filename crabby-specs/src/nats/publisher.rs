use std::sync::Arc;

use async_nats::Client;
use async_trait::async_trait;
use crabby_transport::{
    channel::Channel,
    codec::{Codec, MsgpackCodec},
    publisher::Publisher,
};
use eyre::{Ok, Result};

use crate::ws::outgoing::CrabbyWsFromServer;

pub struct NatsCorePublisher {
    inner: Client,
    subject: String,
}
#[async_trait]
impl<C> Publisher<C> for NatsCorePublisher
where
    C: Channel + Send + Sync + 'static,
    C::Message: Send + Sync,
{
    async fn publish(&self, message: C::Message) -> Result<()> {
        let encoded = C::Codec::encode(&message)?;
        //TODO: Use something other than clone()
        let _ = self.inner.publish(self.subject.clone(), encoded).await?;
        Ok(())
    }
}

impl NatsCorePublisher {
    pub fn new(client: Client, channel: &impl Channel) -> Self {
        Self {
            inner: client,
            subject: channel.subject(),
        }
    }
}


