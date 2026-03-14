use axum::{
    Json,
    http::StatusCode,
    response::{Response, Result},
};
use axum_extra::routing::{RouterExt, TypedPath};
use into_response::IntoResponse;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize, Serialize, Debug)]
#[serde(transparent)]

pub(crate) struct GroupId(Uuid);

impl ToString for GroupId {
    fn to_string(&self) -> String {
        self.0.to_string()
    }
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(transparent)]
pub(crate) struct MemberId(Uuid);

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
    creator_id: Uuid,
    group_members: Vec<MemberId>,
}

async fn create_group(
    _: CreateGroupRequest,
    Json(create_request): Json<CreateGroupPayload>,
) -> Result<GroupId> {
    todo!()
}

#[derive(TypedPath, Deserialize)]
#[typed_path("/group/{group_id}/add")]
pub(crate) struct AddUserToGroupRequest {
    group_id: GroupId,
}

pub(crate) struct AddUserToGroupPayload {
    actor_id: MemberId,
    new_member_id: MemberId,
}

async fn add_user(
    AddUserToGroupRequest { group_id }: AddUserToGroupRequest,
    Json(payload): Json<AddUserToGroupPayload>,
) -> Result<StatusCode> {
    todo!()
}

#[derive(TypedPath, Deserialize)]
#[typed_path("/group/{group_id}/remove/{user_id}")]
pub(crate) struct RemoveUserFromGroupRequest {
    group_id: GroupId,
    user_id: MemberId,
}

pub(crate) struct RemoveUserFromGroupPayload {
    actor_id: MemberId,
}

async fn remove_user_from_group(
    request: RemoveUserFromGroupRequest,
    Json(actor_id): Json<RemoveUserFromGroupPayload>,
) -> Result<StatusCode> {
    todo!()
}
