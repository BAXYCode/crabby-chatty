use std::str::FromStr;

use async_trait::async_trait;
use crabby_specs::grpc::groups::{
    ListGroupMembersRequest, group_service_client::GroupServiceClient,
};
use eyre::Result;
use uuid::Uuid;

use crate::traits::GroupMembershipClient;

pub struct GrpcGroupClient {
    inner: GroupServiceClient<tonic::transport::Channel>,
}

impl GrpcGroupClient {
    pub fn new(client: GroupServiceClient<tonic::transport::Channel>) -> Self {
        Self { inner: client }
    }
}

#[async_trait]
impl GroupMembershipClient for GrpcGroupClient {
    async fn list_group_members(
        &mut self,
        group_id: Uuid,
    ) -> Result<(Vec<Uuid>, u64)> {
        let req = ListGroupMembersRequest {
            group_id: group_id.to_string(),
        };
        let resp = self.inner.list_group_members(req).await?.into_inner();
        let members = resp
            .user_id
            .into_iter()
            .map(|id| Uuid::from_str(&id))
            .collect::<Result<Vec<_>, _>>()?;
        Ok((members, resp.ver))
    }
}
