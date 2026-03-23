use axum::{http::StatusCode, response::IntoResponse};

#[derive(Debug, thiserror::Error)]
pub enum GroupError {
    #[error("group or member not found")]
    NotFound,

    #[error("not authorized to perform this action")]
    Forbidden,

    #[error("user is already a member of this group")]
    AlreadyMember,

    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
}

impl IntoResponse for GroupError {
    fn into_response(self) -> axum::response::Response {
        match self {
            GroupError::NotFound => StatusCode::NOT_FOUND,
            GroupError::Forbidden => StatusCode::FORBIDDEN,
            GroupError::AlreadyMember => StatusCode::CONFLICT,
            GroupError::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
        .into_response()
    }
}
