use async_trait::async_trait;
use eyre::Result;

use crate::channel::Channel;

#[async_trait]
pub trait Publisher<C: Channel>: Send + Sync + 'static {
    async fn publish(&self, message: C::Message) -> Result<()>;
}
