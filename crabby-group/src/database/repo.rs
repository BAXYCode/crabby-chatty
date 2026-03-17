use core::str;

use eyre::{Ok, Result};
use serde::{Deserialize, Serialize};
use sqlx::{
    Connection, PgPool, Postgres, Transaction, prelude, query, types::Uuid,
};
use tonic::async_trait;

use crate::{
    api::{
        AddUserToGroupPayload, CreateGroupPayload, GroupId, MemberId,
        RemoveUserFromGroupRequest,
    },
    database::models::{Event, Role},
};
#[async_trait]
pub trait DatabaseRepo: Send + Sync {
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
#[async_trait]
impl DatabaseRepo for PgRepo {
    async fn create_group(&self, payload: CreateGroupPayload) -> Result<Uuid> {
        let mut conn = self.conn.acquire().await?;

        let mut tx = conn.begin().await?;

        let record =
            query!("INSERT INTO chat_group DEFAULT VALUES RETURNING group_id")
                .fetch_one(tx.as_mut())
                .await?;

        let group_id = GroupId(record.group_id);

        let _ = query!(
            "INSERT INTO group_membership(group_id, user_id, role) VALUES \
             ($1, $2, $3) ",
            group_id.0,
            payload.creator_id,
            Role::Admin as Role
        )
        .fetch_optional(tx.as_mut())
        .await?;

        for member in payload.group_members.iter() {
            let _ = self.add_to_group(&mut tx, &group_id, member).await;
        }

        let _ = tx.commit().await?;

        Ok(group_id.0)
    }

    async fn add_user_to_group(
        &self,
        payload: AddUserToGroupPayload,
        group_id: GroupId,
    ) -> Result<bool> {
        let mut tx = self.conn.begin().await?;

        let _ = query!(
            "INSERT INTO group_membership(group_id, user_id, role) VALUES \
             ($1, $2, $3) ",
            group_id.0,
            payload.new_member_id.0,
            Role::Admin as Role
        )
        .execute(tx.as_mut())
        .await?;

        let _ = query!(
            "INSERT INTO group_event(subject_id, actor_id, group_id, \
             event_type) VALUES ($1, $2, $3, $4) ",
            payload.new_member_id.0,
            payload.actor_id.0,
            group_id.0,
            Event::Added as Event
        )
        .execute(tx.as_mut())
        .await?;
        let _ = tx.commit().await?;
        Ok(true)
    }

    async fn remove_user_from_group(
        &self,
        actor: MemberId,
        payload: RemoveUserFromGroupRequest,
    ) -> Result<bool> {
        let mut tx = self.conn.begin().await?;

        let _ = query!(
            "DELETE FROM group_membership WHERE group_id = $1 and user_id = $2",
            payload.group_id.0,
            payload.user_id.0
        )
        .execute(tx.as_mut())
        .await?;
        if actor.0 == payload.user_id.0 {
            let _ = query!(
                "INSERT INTO group_event(subject_id, actor_id, group_id, \
                 event_type) VALUES ($1, $2, $3, $4) ",
                payload.user_id.0,
                actor.0,
                payload.group_id.0,
                Event::Left as Event
            )
            .execute(tx.as_mut())
            .await?;
        }

        let _ = query!(
            "INSERT INTO group_event(subject_id, actor_id, group_id, \
             event_type) VALUES ($1, $2, $3, $4) ",
            payload.user_id.0,
            actor.0,
            payload.group_id.0,
            Event::Removed as Event
        )
        .execute(tx.as_mut())
        .await?;
        tx.commit().await?;
        Ok(true)
    }
}

impl PgRepo {
    async fn add_to_group(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        group_id: &GroupId,
        new_member: &MemberId,
    ) -> Result<()> {
        let _ = query!(
            "INSERT INTO group_membership(group_id, user_id, role) VALUES \
             ($1,$2,$3)",
            group_id.0,
            new_member.0,
            Role::Member as Role,
        )
        .fetch_optional(tx.as_mut())
        .await?;

        Ok(())
    }
}
