use core::str;

use eyre::Result;
use sqlx::PgPool;
use uuid::Uuid;

use crate::api::{
    AddUserToGroupPayload, CreateGroupPayload, GroupId, MemberId,
    RemoveUserFromGroupRequest,
};

trait DatabaseRepo: Send + Sync {
    async fn create_group(&self, payload: CreateGroupPayload) -> Result<Uuid>;
    async fn add_user_to_group(
        &self,
        payload: AddUserToGroupPayload,
        group_id: GroupId,
    ) -> Result<bool>;
    async fn remove_user_from_group(
        &self,
        actor: MemberId,
        payload: RemoveUserFromGroupRequest,
    ) -> Result<bool>;
}

pub struct PgRepo {
    conn: PgPool,
}

impl DatabaseRepo for PgRepo {
    async fn create_group(&self, payload: CreateGroupPayload) -> Result<Uuid> {
        todo!()
    }

    async fn add_user_to_group(
        &self,
        payload: AddUserToGroupPayload,
        group_id: GroupId,
    ) -> Result<bool> {
        todo!()
    }

    async fn remove_user_from_group(
        &self,
        actor: MemberId,
        payload: RemoveUserFromGroupRequest,
    ) -> Result<bool> {
        todo!()
    }
}
