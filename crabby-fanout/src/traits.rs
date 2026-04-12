use async_trait::async_trait;
use crabby_specs::ws::outgoing::CrabbyWsFromServer;
use eyre::Result;
use uuid::Uuid;

#[async_trait]
pub trait GroupMembershipClient: Send + Sync + 'static {
    /// Returns the list of member UUIDs and the group version.
    async fn list_group_members(
        &mut self,
        group_id: Uuid,
    ) -> Result<(Vec<Uuid>, u64)>;
}

#[async_trait]
pub trait UserMessagePublisher: Send + Sync + 'static {
    async fn publish_to_user(
        &self,
        recipient_id: Uuid,
        message: CrabbyWsFromServer,
    ) -> Result<()>;
}
