use async_nats::Client;
use crabby_transport::{channel::Channel, transport::Transport};
use eyre::{Ok, Result};

use crate::nats::{
    publisher::NatsCorePublisher, subscriber::NatsCoreSubscriber,
};
//TODO:When instantiating the transport, check for transport specific
// URL in the env
// TODO: Take in channel as argument to subscriber and publisher
// methods
pub struct NatsCoreTransport {
    inner: Client,
}

impl<C> Transport<C> for NatsCoreTransport
where
    C: Channel + Send + Sync + 'static,
{
    type Publisher = NatsCorePublisher;

    type Subscriber = NatsCoreSubscriber;

    fn subscriber(&self) -> eyre::Result<Self::Subscriber> {
        Ok(NatsCoreSubscriber::new(self.inner.clone()))
    }

    fn publisher(&self, channel: &C) -> eyre::Result<Self::Publisher> {
        Ok(NatsCorePublisher::new(self.inner.clone(), channel))
    }
}

impl NatsCoreTransport {
    pub async fn new() -> Result<Self> {
        let nats_url = dotenvy::var("NATS_CORE_URL")?;
        Ok(NatsCoreTransport {
            inner: async_nats::connect(nats_url).await?,
        })
    }

    pub fn client(&self) -> &async_nats::Client {
        &self.inner
    }
}
