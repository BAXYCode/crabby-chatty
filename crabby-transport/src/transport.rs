use eyre::Result;

use crate::{channel::Channel, subscriber};

pub trait Transport<C: Channel + Send + Sync + 'static>:
    Send + Sync + 'static
{
    type Publisher;
    type Subscriber;
    fn subscriber(&self) -> Result<Self::Subscriber>;
    fn publisher(&self, channel: &C) -> Result<Self::Publisher>;
}
