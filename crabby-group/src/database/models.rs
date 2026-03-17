use serde::{Deserialize, Serialize};

#[derive(sqlx::Type, Debug, Serialize, Deserialize)]
#[sqlx(type_name = "role", rename_all = "lowercase")]
pub(crate) enum Role {
    Admin,
    Member,
}

#[derive(sqlx::Type, Debug, Serialize, Deserialize)]
#[sqlx(type_name = "event", rename_all = "lowercase")]
pub(crate) enum Event {
    Added,
    Removed,
    Left,
}
