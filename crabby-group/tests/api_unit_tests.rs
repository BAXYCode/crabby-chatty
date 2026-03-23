use std::sync::Arc;

use axum::http::StatusCode;
use axum_test::TestServer;
use crabby_group::{
    api::{
        AddUserToGroupPayload, CreateGroupPayload, MemberId, RemoveUserFromGroupPayload,
        StorageState,
    },
    database::repo::DatabaseRepo,
    error::GroupError,
};
use sqlx::types::Uuid;
use tonic::async_trait;

// `#[automock]` on the trait only fires for intra-crate tests. For external
// integration test binaries we define the mock inline here.
mockall::mock! {
    pub Repo {}

    #[async_trait]
    impl DatabaseRepo for Repo {
        async fn create_group(
            &self,
            payload: crabby_group::api::CreateGroupPayload,
        ) -> Result<Uuid, GroupError>;

        async fn add_user_to_group(
            &self,
            payload: crabby_group::api::AddUserToGroupPayload,
            group_id: crabby_group::api::GroupId,
        ) -> Result<bool, GroupError>;

        async fn remove_user_from_group(
            &self,
            payload: crabby_group::api::RemoveUserFromGroupPayload,
            params: crabby_group::api::RemoveUserFromGroupParams,
        ) -> Result<bool, GroupError>;
    }
}

fn make_server(mock: MockRepo) -> TestServer {
    let state = StorageState {
        store: Arc::new(mock),
    };
    // OpenApiRouter must be split into the plain axum Router before passing to TestServer.
    let (router, _) = crabby_group::api::router().split_for_parts();
    TestServer::new(router.with_state(state)).unwrap()
}

fn rand_uuid() -> Uuid {
    Uuid::new_v4()
}

fn db_err() -> GroupError {
    GroupError::Database(sqlx::Error::RowNotFound)
}

// ── create_group ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn create_group_201_on_success() {
    let mut mock = MockRepo::new();
    mock.expect_create_group()
        .once()
        .returning(|_| Ok(Uuid::new_v4()));

    let server = make_server(mock);
    let res = server
        .post("/group")
        .json(&CreateGroupPayload {
            creator_id: MemberId(rand_uuid()),
            group_members: vec![],
        })
        .await;

    res.assert_status(StatusCode::CREATED);
}

#[tokio::test]
async fn create_group_500_on_db_error() {
    let mut mock = MockRepo::new();
    mock.expect_create_group()
        .once()
        .returning(|_| Err(db_err()));

    let server = make_server(mock);
    let res = server
        .post("/group")
        .json(&CreateGroupPayload {
            creator_id: MemberId(rand_uuid()),
            group_members: vec![],
        })
        .await;

    res.assert_status(StatusCode::INTERNAL_SERVER_ERROR);
}

// ── add_user ──────────────────────────────────────────────────────────────────

#[tokio::test]
async fn add_user_205_on_success() {
    let mut mock = MockRepo::new();
    mock.expect_add_user_to_group()
        .once()
        .returning(|_, _| Ok(true));

    let group_id = rand_uuid();
    let server = make_server(mock);
    let res = server
        .post(&format!("/group/{group_id}/members/{}", rand_uuid()))
        .json(&AddUserToGroupPayload {
            actor_id: MemberId(rand_uuid()),
            new_member_id: MemberId(rand_uuid()),
        })
        .await;

    res.assert_status(StatusCode::RESET_CONTENT);
}

#[tokio::test]
async fn add_user_403_on_forbidden() {
    let mut mock = MockRepo::new();
    mock.expect_add_user_to_group()
        .once()
        .returning(|_, _| Err(GroupError::Forbidden));

    let group_id = rand_uuid();
    let server = make_server(mock);
    let res = server
        .post(&format!("/group/{group_id}/members/{}", rand_uuid()))
        .json(&AddUserToGroupPayload {
            actor_id: MemberId(rand_uuid()),
            new_member_id: MemberId(rand_uuid()),
        })
        .await;

    res.assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn add_user_409_on_already_member() {
    let mut mock = MockRepo::new();
    mock.expect_add_user_to_group()
        .once()
        .returning(|_, _| Err(GroupError::AlreadyMember));

    let group_id = rand_uuid();
    let server = make_server(mock);
    let res = server
        .post(&format!("/group/{group_id}/members/{}", rand_uuid()))
        .json(&AddUserToGroupPayload {
            actor_id: MemberId(rand_uuid()),
            new_member_id: MemberId(rand_uuid()),
        })
        .await;

    res.assert_status(StatusCode::CONFLICT);
}

#[tokio::test]
async fn add_user_500_on_db_error() {
    let mut mock = MockRepo::new();
    mock.expect_add_user_to_group()
        .once()
        .returning(|_, _| Err(db_err()));

    let group_id = rand_uuid();
    let server = make_server(mock);
    let res = server
        .post(&format!("/group/{group_id}/members/{}", rand_uuid()))
        .json(&AddUserToGroupPayload {
            actor_id: MemberId(rand_uuid()),
            new_member_id: MemberId(rand_uuid()),
        })
        .await;

    res.assert_status(StatusCode::INTERNAL_SERVER_ERROR);
}

// ── remove_user ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn remove_user_204_on_success() {
    let mut mock = MockRepo::new();
    mock.expect_remove_user_from_group()
        .once()
        .returning(|_, _| Ok(true));

    let group_id = rand_uuid();
    let member_id = rand_uuid();
    let server = make_server(mock);
    let res = server
        .delete(&format!("/group/{group_id}/members/{member_id}"))
        .json(&RemoveUserFromGroupPayload {
            actor_id: MemberId(rand_uuid()),
        })
        .await;

    res.assert_status(StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn remove_user_403_on_forbidden() {
    let mut mock = MockRepo::new();
    mock.expect_remove_user_from_group()
        .once()
        .returning(|_, _| Err(GroupError::Forbidden));

    let group_id = rand_uuid();
    let member_id = rand_uuid();
    let server = make_server(mock);
    let res = server
        .delete(&format!("/group/{group_id}/members/{member_id}"))
        .json(&RemoveUserFromGroupPayload {
            actor_id: MemberId(rand_uuid()),
        })
        .await;

    res.assert_status(StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn remove_user_404_on_not_found() {
    let mut mock = MockRepo::new();
    mock.expect_remove_user_from_group()
        .once()
        .returning(|_, _| Err(GroupError::NotFound));

    let group_id = rand_uuid();
    let member_id = rand_uuid();
    let server = make_server(mock);
    let res = server
        .delete(&format!("/group/{group_id}/members/{member_id}"))
        .json(&RemoveUserFromGroupPayload {
            actor_id: MemberId(rand_uuid()),
        })
        .await;

    res.assert_status(StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn remove_user_500_on_db_error() {
    let mut mock = MockRepo::new();
    mock.expect_remove_user_from_group()
        .once()
        .returning(|_, _| Err(db_err()));

    let group_id = rand_uuid();
    let member_id = rand_uuid();
    let server = make_server(mock);
    let res = server
        .delete(&format!("/group/{group_id}/members/{member_id}"))
        .json(&RemoveUserFromGroupPayload {
            actor_id: MemberId(rand_uuid()),
        })
        .await;

    res.assert_status(StatusCode::INTERNAL_SERVER_ERROR);
}
