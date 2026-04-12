use async_trait::async_trait;
use crabby_specs::{
    nats::{channel::UserMessageDelivery, transport::NatsCoreTransport},
    ws::outgoing::CrabbyWsFromServer,
};
use crabby_transport::{publisher::Publisher, transport::Transport};
use eyre::Result;
use uuid::Uuid;

use crate::traits::UserMessagePublisher;

pub struct NatsUserMessagePublisher {
    transport: NatsCoreTransport,
}

impl NatsUserMessagePublisher {
    pub fn new(transport: NatsCoreTransport) -> Self {
        Self { transport }
    }
}

#[async_trait]
impl UserMessagePublisher for NatsUserMessagePublisher {
    async fn publish_to_user(
        &self,
        recipient_id: Uuid,
        message: CrabbyWsFromServer,
    ) -> Result<()> {
        let channel = UserMessageDelivery::new(&recipient_id.to_string());
        let publisher =
            Transport::<UserMessageDelivery>::publisher(&self.transport, &channel)?;
        Publisher::<UserMessageDelivery>::publish(&publisher, message).await
    }
}
