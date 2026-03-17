use std::sync::Arc;

use axum::{
    Error, Json,
    extract::{FromRef, State},
    http::StatusCode,
    response::{Response, Result},
};
use axum_extra::routing::{RouterExt, TypedPath};
use into_response::IntoResponse;
use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;

use crate::database::repo::DatabaseRepo;

#[derive(FromRef)]
pub struct StorageState {
    pub store: Arc<dyn DatabaseRepo>,
}

#[derive(Deserialize, Serialize, Debug, sqlx::Type)]
#[serde(transparent)]
#[sqlx(transparent)]
pub(crate) struct GroupId(pub Uuid);

impl ToString for GroupId {
    fn to_string(&self) -> String {
        self.0.to_string()
    }
}

#[derive(Deserialize, Serialize, Debug, sqlx::Type)]
#[serde(transparent)]
#[sqlx(transparent)]
pub(crate) struct MemberId(pub Uuid);

impl ToString for MemberId {
    fn to_string(&self) -> String {
        self.0.to_string()
    }
}

#[derive(TypedPath, Deserialize)]
#[typed_path("/group/create")]
struct CreateGroupRequest;

#[derive(Deserialize)]
pub(crate) struct CreateGroupPayload {
    pub creator_id: Uuid,
    pub group_members: Vec<MemberId>,
}

async fn create_group(
    State(state): State<StorageState>,
    _: CreateGroupRequest,
    Json(create_request): Json<CreateGroupPayload>,
) -> Result<GroupId> {
    let group_id = state
        .store
        .create_group(create_request)
        .await
        .map_err(|e| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(GroupId(group_id))
}

#[derive(TypedPath, Deserialize)]
#[typed_path("/group/{group_id}/add")]

pub(crate) struct AddUserToGroupRequest {
    pub group_id: GroupId,
}

pub(crate) struct AddUserToGroupPayload {
    pub actor_id: MemberId,
    pub new_member_id: MemberId,
}

async fn add_user(
    State(state): State<StorageState>,
    AddUserToGroupRequest { group_id }: AddUserToGroupRequest,
    Json(payload): Json<AddUserToGroupPayload>,
) -> Result<bool> {
    let done = state
        .store
        .add_user_to_group(payload, group_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(true)
}

#[derive(TypedPath, Deserialize)]
#[typed_path("/group/{group_id}/remove/{user_id}")]

pub(crate) struct RemoveUserFromGroupRequest {
    pub group_id: GroupId,
    pub user_id: MemberId,
}

pub(crate) struct RemoveUserFromGroupPayload {
    pub actor_id: MemberId,
}

async fn remove_user_from_group(
    State(state): State<StorageState>,
    request: RemoveUserFromGroupRequest,
    Json(payload): Json<RemoveUserFromGroupPayload>,
) -> Result<bool> {
    let done = state
        .store
        .remove_user_from_group(payload.actor_id, request)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(done)
}
