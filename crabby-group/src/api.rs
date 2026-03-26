use std::sync::Arc;

use axum::{
    Json,
    extract::{FromRef, Path, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{database::repo::DatabaseRepo, error::GroupError};

#[derive(FromRef, Clone)]
pub struct StorageState {
    pub store: Arc<dyn DatabaseRepo>,
}

#[derive(Deserialize, Serialize, Debug, sqlx::Type, ToSchema)]
#[serde(transparent)]
#[sqlx(transparent)]
pub struct GroupId(pub Uuid);

impl ToString for GroupId {
    fn to_string(&self) -> String {
        self.0.to_string()
    }
}

#[derive(Deserialize, Serialize, Debug, sqlx::Type, ToSchema)]
#[serde(transparent)]
#[sqlx(transparent)]
pub struct MemberId(pub Uuid);

impl ToString for MemberId {
    fn to_string(&self) -> String {
        self.0.to_string()
    }
}

#[derive(ToSchema, Deserialize, Debug, Serialize)]
pub struct CreateGroupPayload {
    pub creator_id: MemberId,
    pub group_members: Vec<MemberId>,
}

#[utoipa::path(
    post,
    path = "/group",
    responses(
        (status = 201, description = "Group created succesfully"),
        (status = 500, description = "Internal server error")
    ))]
async fn create_group(
    State(state): State<StorageState>,
    Json(create_request): Json<CreateGroupPayload>,
) -> Result<(StatusCode, Json<Option<GroupId>>), GroupError> {
    let id = state.store.create_group(create_request).await?;
    Ok((StatusCode::CREATED, Json(Some(GroupId(id)))))
}

#[derive(ToSchema, Deserialize, IntoParams, Serialize)]
pub struct AddUserToGroupParams {
    pub group_id: GroupId,
}
#[derive(ToSchema, Deserialize, Serialize, Debug)]
pub struct AddUserToGroupPayload {
    pub actor_id: MemberId,
    pub new_member_id: MemberId,
}

#[utoipa::path(
    post,
    path = "/group/{group_id}/members/{member_id}",
    params(AddUserToGroupParams),
    responses(
        (status = 204, description = "User added successfully"),
        (status = 404, description = "Membership not found"),
        (status = 409, description = "User is already a member of the group"),
        (status = 500, description = "Internal server error")
    ))]
async fn add_user(
    State(state): State<StorageState>,
    Path(AddUserToGroupParams { group_id }): Path<AddUserToGroupParams>,
    Json(payload): Json<AddUserToGroupPayload>,
) -> Result<StatusCode, GroupError> {
    state.store.add_user_to_group(payload, group_id).await?;
    Ok(StatusCode::RESET_CONTENT)
}

#[derive(IntoParams, Deserialize, Serialize)]
pub struct RemoveUserFromGroupParams {
    pub group_id: GroupId,
    pub member_id: MemberId,
}

#[derive(ToSchema, Deserialize, Serialize)]
pub struct RemoveUserFromGroupPayload {
    pub actor_id: MemberId,
}
#[utoipa::path(
    delete,
    path = "/group/{group_id}/members/{member_id}",
    params(RemoveUserFromGroupParams),
    responses(
        (status = 204, description = "User removed successfully"),
        (status = 404, description = "Membership not found"),
        (status = 403, description = "Not allowed to remove this user"),
        (status = 500, description = "Internal server error"))
)]
async fn remove_user(
    State(state): State<StorageState>,
    Path(request): Path<RemoveUserFromGroupParams>,
    Json(payload): Json<RemoveUserFromGroupPayload>,
) -> Result<StatusCode, GroupError> {
    state.store.remove_user_from_group(payload, request).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub fn router() -> OpenApiRouter<StorageState> {
    OpenApiRouter::new()
        .routes(routes!(create_group))
        .routes(routes!(add_user, remove_user))
}
