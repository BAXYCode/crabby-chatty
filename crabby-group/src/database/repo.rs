use sqlx::{PgPool, Postgres, Transaction, query, types::Uuid};
use tonic::async_trait;

use crate::{
    api::{
        AddUserToGroupPayload, CreateGroupPayload, GroupId, MemberId,
        RemoveUserFromGroupParams, RemoveUserFromGroupPayload,
    },
    database::models::{Event, Role},
    error::GroupError,
};

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait DatabaseRepo: Send + Sync {
    async fn create_group(
        &self,
        payload: CreateGroupPayload,
    ) -> Result<Uuid, GroupError>;

    async fn add_user_to_group(
        &self,
        payload: AddUserToGroupPayload,
        group_id: GroupId,
    ) -> Result<bool, GroupError>;

    async fn remove_user_from_group(
        &self,
        payload: RemoveUserFromGroupPayload,
        params: RemoveUserFromGroupParams,
    ) -> Result<bool, GroupError>;
}

pub struct PgRepo {
    conn: PgPool,
}

#[async_trait]
impl DatabaseRepo for PgRepo {
    async fn create_group(
        &self,
        payload: CreateGroupPayload,
    ) -> Result<Uuid, GroupError> {
        let mut tx = self.conn.begin().await?;

        let record =
            query!("INSERT INTO chat_group DEFAULT VALUES RETURNING group_id")
                .fetch_one(tx.as_mut())
                .await?;

        let group_id = GroupId(record.group_id);

        let _ = query!(
            "INSERT INTO group_membership(group_id, user_id, role) VALUES \
             ($1, $2, $3)",
            group_id.0,
            payload.creator_id.0,
            Role::Admin as Role
        )
        .fetch_optional(tx.as_mut())
        .await?;

        // Deduplicate before inserting: a duplicate inside the transaction
        // would abort the whole transaction (PostgreSQL marks it as failed),
        // so we must never attempt a duplicate insert. Pre-seed with the
        // creator so that a member ID matching the creator is also skipped.
        let mut seen = std::collections::HashSet::new();
        seen.insert(payload.creator_id.0);
        for member in payload.group_members.iter() {
            if seen.insert(member.0) {
                let _ = self.add_to_group(&mut tx, &group_id, member).await;
            }
        }

        let _ = tx.commit().await?;

        Ok(group_id.0)
    }

    async fn add_user_to_group(
        &self,
        payload: AddUserToGroupPayload,
        group_id: GroupId,
    ) -> Result<bool, GroupError> {
        let mut tx = self.conn.begin().await?;

        // Actor must be an admin to add users
        let actor_role = query!(
            "SELECT role as \"role: Role\" FROM group_membership WHERE \
             group_id = $1 AND user_id = $2",
            group_id.0,
            payload.actor_id.0
        )
        .fetch_optional(tx.as_mut())
        .await?;

        let is_admin = actor_role
            .and_then(|r| r.role)
            .map(|role| role == Role::Admin)
            .unwrap_or(false);
        if !is_admin {
            return Err(GroupError::Forbidden);
        }

        query!(
            "INSERT INTO group_membership(group_id, user_id, role) VALUES \
             ($1, $2, $3)",
            group_id.0,
            payload.new_member_id.0,
            Role::Member as Role
        )
        .execute(tx.as_mut())
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(ref db_err) = e {
                if db_err.kind() == sqlx::error::ErrorKind::UniqueViolation {
                    return GroupError::AlreadyMember;
                }
            }
            GroupError::Database(e)
        })?;

        query!(
            "INSERT INTO group_event(subject_id, actor_id, group_id, \
             event_type) VALUES ($1, $2, $3, $4)",
            payload.new_member_id.0,
            payload.actor_id.0,
            group_id.0,
            Event::Added as Event
        )
        .execute(tx.as_mut())
        .await?;

        tx.commit().await?;
        Ok(true)
    }

    async fn remove_user_from_group(
        &self,
        payload: RemoveUserFromGroupPayload,
        params: RemoveUserFromGroupParams,
    ) -> Result<bool, GroupError> {
        let mut tx = self.conn.begin().await?;

        // Check actor's role in the group
        let actor_role = query!(
            "SELECT role as \"role: Role\" FROM group_membership WHERE \
             group_id = $1 AND user_id = $2",
            params.group_id.0,
            payload.actor_id.0
        )
        .fetch_optional(tx.as_mut())
        .await?;

        let is_self_remove = params.member_id.0 == payload.actor_id.0;
        let is_admin = actor_role
            .and_then(|r| r.role)
            .map(|role| role == Role::Admin)
            .unwrap_or(false);

        if !is_self_remove && !is_admin {
            return Err(GroupError::Forbidden);
        }

        let result = query!(
            "DELETE FROM group_membership WHERE group_id = $1 AND user_id = $2",
            params.group_id.0,
            params.member_id.0
        )
        .execute(tx.as_mut())
        .await?;

        if result.rows_affected() == 0 {
            return Err(GroupError::NotFound);
        }

        if is_self_remove {
            query!(
                "INSERT INTO group_event(subject_id, actor_id, group_id, \
                 event_type) VALUES ($1, $2, $3, $4)",
                params.member_id.0,
                payload.actor_id.0,
                params.group_id.0,
                Event::Left as Event
            )
            .execute(tx.as_mut())
            .await?;
        } else {
            query!(
                "INSERT INTO group_event(subject_id, actor_id, group_id, \
                 event_type) VALUES ($1, $2, $3, $4)",
                params.member_id.0,
                payload.actor_id.0,
                params.group_id.0,
                Event::Removed as Event
            )
            .execute(tx.as_mut())
            .await?;
        }

        tx.commit().await?;
        Ok(true)
    }
}

impl PgRepo {
    pub fn new(conn: PgPool) -> Self {
        Self { conn }
    }

    async fn add_to_group(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        group_id: &GroupId,
        new_member: &MemberId,
    ) -> Result<(), GroupError> {
        let _ = query!(
            "INSERT INTO group_membership(group_id, user_id, role) VALUES \
             ($1, $2, $3)",
            group_id.0,
            new_member.0,
            Role::Member as Role,
        )
        .fetch_optional(tx.as_mut())
        .await?;

        Ok(())
    }
}
