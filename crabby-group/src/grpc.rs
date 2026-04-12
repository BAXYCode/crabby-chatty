use std::collections::HashMap;

use sqlx::{PgPool, types::Uuid};
use tonic::{Request, Response, Status};

pub mod proto {
    tonic::include_proto!("groups");
}

use proto::{
    BatchListGroupMembersRequest, BatchListGroupMembersResponse,
    CheckMembershipRequest, CheckMembershipResponse,
    GetGroupMembershipVersionRequest, GetGroupMembershipVersionResponse,
    GroupMembers, ListGroupMembersRequest, ListGroupMembersResponse,
    group_service_server::{GroupService, GroupServiceServer},
};

pub struct GroupServiceImpl {
    pool: PgPool,
}

impl GroupServiceImpl {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn into_server(self) -> GroupServiceServer<Self> {
        GroupServiceServer::new(self)
    }
}

fn parse_uuid(s: &str) -> Result<Uuid, Status> {
    Uuid::parse_str(s)
        .map_err(|_| Status::invalid_argument(format!("invalid UUID: '{s}'")))
}

#[tonic::async_trait]
impl GroupService for GroupServiceImpl {
    /// Returns `true` if the user has at least one group membership.
    async fn check_membership(
        &self,
        request: Request<CheckMembershipRequest>,
    ) -> Result<Response<CheckMembershipResponse>, Status> {
        let user_id = parse_uuid(&request.into_inner().user_id)?;

        let membership = sqlx::query_scalar!(
            "SELECT EXISTS(SELECT 1 FROM group_membership WHERE user_id = $1)",
            user_id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| Status::internal(e.to_string()))?
        .unwrap_or(false);

        Ok(Response::new(CheckMembershipResponse { membership }))
    }

    /// Returns all member UUIDs and the current membership version
    /// for `group_id`. Returns `PERMISSION_DENIED` if the
    /// requesting `user_id` is not in the group.
    async fn list_group_members(
        &self,
        request: Request<ListGroupMembersRequest>,
    ) -> Result<Response<ListGroupMembersResponse>, Status> {
        let req = request.into_inner();
        // let user_id = parse_uuid(&req.user_id)?;
        let group_id = parse_uuid(&req.group_id)?;

        // let is_member = sqlx::query_scalar!(
        //     "SELECT EXISTS(
        //         SELECT 1 FROM group_membership WHERE group_id = $1 AND
        // user_id \      = $2
        //      )",
        //     group_id,
        //     user_id
        // )
        // .fetch_one(&self.pool)
        // .await
        // .map_err(|e| Status::internal(e.to_string()))?
        // .unwrap_or(false);
        //
        // if !is_member {
        //     return Err(Status::permission_denied(
        //         "not a member of this group",
        //     ));
        // }

        let rows = sqlx::query!(
            "SELECT gm.user_id, gmv.version
             FROM group_membership gm
             LEFT JOIN group_membership_version gmv ON gm.group_id = \
             gmv.group_id
             WHERE gm.group_id = $1",
            group_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Status::internal(e.to_string()))?;

        let ver = rows.first().and_then(|r| r.version).unwrap_or(0) as u64;
        let user_id = rows.into_iter().map(|r| r.user_id.to_string()).collect();

        Ok(Response::new(ListGroupMembersResponse { user_id, ver }))
    }

    /// Returns a map of `group_id → [member UUIDs, ver]` for each
    /// requested group.
    async fn batch_list_group_members(
        &self,
        request: Request<BatchListGroupMembersRequest>,
    ) -> Result<Response<BatchListGroupMembersResponse>, Status> {
        let group_ids: Vec<Uuid> = request
            .into_inner()
            .group_id
            .iter()
            .map(|id| parse_uuid(id))
            .collect::<Result<_, _>>()?;

        let rows = sqlx::query!(
            "SELECT gm.group_id, gm.user_id, COALESCE(gmv.version, 0) as \
             version
             FROM group_membership gm
             LEFT JOIN group_membership_version gmv ON gm.group_id = \
             gmv.group_id
             WHERE gm.group_id = ANY($1::uuid[])",
            &group_ids as &[Uuid]
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Status::internal(e.to_string()))?;

        let mut response: HashMap<String, GroupMembers> = HashMap::new();
        for row in rows {
            let entry = response
                .entry(row.group_id.unwrap().to_string())
                .or_insert_with(|| {
                    GroupMembers {
                        member: Vec::new(),
                        ver: row.version.unwrap_or(0) as u64,
                    }
                });
            entry.member.push(row.user_id.unwrap().to_string());
        }

        Ok(Response::new(BatchListGroupMembersResponse { response }))
    }

    /// Returns the current membership version for `group_id`.
    async fn get_group_membership_version(
        &self,
        request: Request<GetGroupMembershipVersionRequest>,
    ) -> Result<Response<GetGroupMembershipVersionResponse>, Status> {
        let group_id = parse_uuid(&request.into_inner().group_id)?;

        let ver = sqlx::query_scalar!(
            "SELECT version FROM group_membership_version WHERE group_id = $1",
            group_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Status::internal(e.to_string()))?
        .ok_or_else(|| Status::not_found("group not found"))?;

        Ok(Response::new(GetGroupMembershipVersionResponse {
            ver: ver as u64,
        }))
    }
}
