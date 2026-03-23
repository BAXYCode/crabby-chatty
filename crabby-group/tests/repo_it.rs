mod common;

use crabby_group::{
    api::{AddUserToGroupPayload, CreateGroupPayload, GroupId, MemberId, RemoveUserFromGroupParams,
          RemoveUserFromGroupPayload},
    database::repo::{DatabaseRepo, PgRepo},
    error::GroupError,
};
use sqlx::types::Uuid;
use uuid::uuid;

// Fixed UUIDs matching the fixture files
const GROUP_1: Uuid = uuid!("11111111-0000-0000-0000-000000000001");
const GROUP_2: Uuid = uuid!("22222222-0000-0000-0000-000000000001");
const ADMIN_1: Uuid = uuid!("aaaaaaaa-0000-0000-0000-000000000001");
const MEMBER_1: Uuid = uuid!("bbbbbbbb-0000-0000-0000-000000000002");

fn group_id(id: Uuid) -> GroupId {
    GroupId(id)
}
fn member_id(id: Uuid) -> MemberId {
    MemberId(id)
}

// ── create_group ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn create_group_returns_uuid() -> eyre::Result<()> {
    let db = common::TestDb::new().await?;
    let repo = PgRepo::new(db.pool.clone());

    let result = repo
        .create_group(CreateGroupPayload {
            creator_id: member_id(Uuid::new_v4()),
            group_members: vec![],
        })
        .await;

    assert!(result.is_ok());
    db.teardown().await
}

#[tokio::test]
async fn create_group_creator_becomes_admin() -> eyre::Result<()> {
    let db = common::TestDb::new().await?;
    let repo = PgRepo::new(db.pool.clone());
    let creator = Uuid::new_v4();

    let group_uuid = repo
        .create_group(CreateGroupPayload {
            creator_id: member_id(creator),
            group_members: vec![],
        })
        .await?;

    // If creator is admin, adding a new user as that actor should succeed
    let new_user = Uuid::new_v4();
    let add_result = repo
        .add_user_to_group(
            AddUserToGroupPayload {
                actor_id: member_id(creator),
                new_member_id: member_id(new_user),
            },
            group_id(group_uuid),
        )
        .await;

    assert!(add_result.is_ok(), "creator should have admin role");
    db.teardown().await
}

#[tokio::test]
async fn create_group_members_get_member_role() -> eyre::Result<()> {
    let db = common::TestDb::new().await?;
    let repo = PgRepo::new(db.pool.clone());
    let creator = Uuid::new_v4();
    let extra = Uuid::new_v4();

    let group_uuid = repo
        .create_group(CreateGroupPayload {
            creator_id: member_id(creator),
            group_members: vec![member_id(extra)],
        })
        .await?;

    let row = sqlx::query!(
        "SELECT role as \"role: String\" FROM group_membership WHERE group_id = $1 AND user_id = $2",
        group_uuid,
        extra
    )
    .fetch_one(&db.pool)
    .await?;

    assert_eq!(row.role.as_deref(), Some("member"));
    db.teardown().await
}

#[tokio::test]
async fn create_group_duplicate_member_ids_silently_ignored() -> eyre::Result<()> {
    let db = common::TestDb::new().await?;
    let repo = PgRepo::new(db.pool.clone());
    let creator = Uuid::new_v4();
    let dup = Uuid::new_v4();

    let group_uuid = repo
        .create_group(CreateGroupPayload {
            creator_id: member_id(creator),
            group_members: vec![member_id(dup), member_id(dup)],
        })
        .await?;

    let count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM group_membership WHERE group_id = $1",
        group_uuid
    )
    .fetch_one(&db.pool)
    .await?;

    // creator (admin) + 1 unique extra member = 2
    assert_eq!(count, Some(2));
    db.teardown().await
}

// ── add_user_to_group ─────────────────────────────────────────────────────────

#[tokio::test]
async fn add_user_succeeds_as_admin() -> eyre::Result<()> {
    let db = common::TestDb::new()
        .await?
        .with_fixture(include_str!("fixtures/group_admin_only.sql"))
        .await?;
    let repo = PgRepo::new(db.pool.clone());
    let new_user = Uuid::new_v4();

    let result = repo
        .add_user_to_group(
            AddUserToGroupPayload {
                actor_id: member_id(ADMIN_1),
                new_member_id: member_id(new_user),
            },
            group_id(GROUP_2),
        )
        .await;

    assert!(result.is_ok());
    db.teardown().await
}

#[tokio::test]
async fn add_user_forbidden_as_member() -> eyre::Result<()> {
    let db = common::TestDb::new()
        .await?
        .with_fixture(include_str!("fixtures/group_with_members.sql"))
        .await?;
    let repo = PgRepo::new(db.pool.clone());

    let result = repo
        .add_user_to_group(
            AddUserToGroupPayload {
                actor_id: member_id(MEMBER_1),
                new_member_id: member_id(Uuid::new_v4()),
            },
            group_id(GROUP_1),
        )
        .await;

    assert!(matches!(result, Err(GroupError::Forbidden)));
    db.teardown().await
}

#[tokio::test]
async fn add_user_forbidden_as_non_member() -> eyre::Result<()> {
    let db = common::TestDb::new()
        .await?
        .with_fixture(include_str!("fixtures/group_admin_only.sql"))
        .await?;
    let repo = PgRepo::new(db.pool.clone());

    let result = repo
        .add_user_to_group(
            AddUserToGroupPayload {
                actor_id: member_id(Uuid::new_v4()),
                new_member_id: member_id(Uuid::new_v4()),
            },
            group_id(GROUP_2),
        )
        .await;

    assert!(matches!(result, Err(GroupError::Forbidden)));
    db.teardown().await
}

#[tokio::test]
async fn add_user_conflict_when_already_member() -> eyre::Result<()> {
    let db = common::TestDb::new()
        .await?
        .with_fixture(include_str!("fixtures/group_with_members.sql"))
        .await?;
    let repo = PgRepo::new(db.pool.clone());

    let result = repo
        .add_user_to_group(
            AddUserToGroupPayload {
                actor_id: member_id(ADMIN_1),
                new_member_id: member_id(MEMBER_1),
            },
            group_id(GROUP_1),
        )
        .await;

    assert!(matches!(result, Err(GroupError::AlreadyMember)));
    db.teardown().await
}

#[tokio::test]
async fn add_user_writes_outbox_row() -> eyre::Result<()> {
    let db = common::TestDb::new()
        .await?
        .with_fixture(include_str!("fixtures/group_admin_only.sql"))
        .await?;
    let repo = PgRepo::new(db.pool.clone());
    let new_user = Uuid::new_v4();

    repo.add_user_to_group(
        AddUserToGroupPayload {
            actor_id: member_id(ADMIN_1),
            new_member_id: member_id(new_user),
        },
        group_id(GROUP_2),
    )
    .await?;

    let count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM outbox WHERE group_id = $1 AND event_type = 'added'",
        GROUP_2
    )
    .fetch_one(&db.pool)
    .await?;

    assert_eq!(count, Some(1));
    db.teardown().await
}

// ── remove_user_from_group ────────────────────────────────────────────────────

#[tokio::test]
async fn remove_user_succeeds_as_admin() -> eyre::Result<()> {
    let db = common::TestDb::new()
        .await?
        .with_fixture(include_str!("fixtures/group_with_members.sql"))
        .await?;
    let repo = PgRepo::new(db.pool.clone());

    let result = repo
        .remove_user_from_group(
            RemoveUserFromGroupPayload {
                actor_id: member_id(ADMIN_1),
            },
            RemoveUserFromGroupParams {
                group_id: group_id(GROUP_1),
                member_id: member_id(MEMBER_1),
            },
        )
        .await;

    assert!(result.is_ok());

    let exists = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM group_membership WHERE group_id = $1 AND user_id = $2",
        GROUP_1, MEMBER_1
    )
    .fetch_one(&db.pool)
    .await?;
    assert_eq!(exists, Some(0));

    db.teardown().await
}

#[tokio::test]
async fn remove_user_self_remove_succeeds() -> eyre::Result<()> {
    let db = common::TestDb::new()
        .await?
        .with_fixture(include_str!("fixtures/group_with_members.sql"))
        .await?;
    let repo = PgRepo::new(db.pool.clone());

    let result = repo
        .remove_user_from_group(
            RemoveUserFromGroupPayload {
                actor_id: member_id(MEMBER_1),
            },
            RemoveUserFromGroupParams {
                group_id: group_id(GROUP_1),
                member_id: member_id(MEMBER_1),
            },
        )
        .await;

    assert!(result.is_ok());

    let count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM outbox WHERE group_id = $1 AND event_type = 'left'",
        GROUP_1
    )
    .fetch_one(&db.pool)
    .await?;
    assert_eq!(count, Some(1));

    db.teardown().await
}

#[tokio::test]
async fn remove_user_forbidden_member_removing_other() -> eyre::Result<()> {
    let db = common::TestDb::new()
        .await?
        .with_fixture(include_str!("fixtures/group_with_members.sql"))
        .await?;
    let repo = PgRepo::new(db.pool.clone());

    let result = repo
        .remove_user_from_group(
            RemoveUserFromGroupPayload {
                actor_id: member_id(MEMBER_1),
            },
            RemoveUserFromGroupParams {
                group_id: group_id(GROUP_1),
                member_id: member_id(ADMIN_1),
            },
        )
        .await;

    assert!(matches!(result, Err(GroupError::Forbidden)));
    db.teardown().await
}

#[tokio::test]
async fn remove_user_not_found_when_absent() -> eyre::Result<()> {
    let db = common::TestDb::new()
        .await?
        .with_fixture(include_str!("fixtures/group_admin_only.sql"))
        .await?;
    let repo = PgRepo::new(db.pool.clone());

    let result = repo
        .remove_user_from_group(
            RemoveUserFromGroupPayload {
                actor_id: member_id(ADMIN_1),
            },
            RemoveUserFromGroupParams {
                group_id: group_id(GROUP_2),
                member_id: member_id(Uuid::new_v4()),
            },
        )
        .await;

    assert!(matches!(result, Err(GroupError::NotFound)));
    db.teardown().await
}

#[tokio::test]
async fn remove_user_admin_writes_removed_event() -> eyre::Result<()> {
    let db = common::TestDb::new()
        .await?
        .with_fixture(include_str!("fixtures/group_with_members.sql"))
        .await?;
    let repo = PgRepo::new(db.pool.clone());

    repo.remove_user_from_group(
        RemoveUserFromGroupPayload {
            actor_id: member_id(ADMIN_1),
        },
        RemoveUserFromGroupParams {
            group_id: group_id(GROUP_1),
            member_id: member_id(MEMBER_1),
        },
    )
    .await?;

    let count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM outbox WHERE group_id = $1 AND event_type = 'removed'",
        GROUP_1
    )
    .fetch_one(&db.pool)
    .await?;
    assert_eq!(count, Some(1));

    db.teardown().await
}
