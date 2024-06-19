use sqlx::prelude::FromRow;
use sqlx::types::chrono::{DateTime, Local as LocalTime};
use uuid::Uuid;
#[derive(FromRow, Debug)]
struct RowId(i64);
#[derive(FromRow, Debug)]
pub(crate) struct SaltDb {
    pub id: i64,
    pub salt: Uuid,
}
#[derive(FromRow)]
pub(crate) struct PasswordDb {
    pub id: i64,
    pub password: String,
}
#[derive(FromRow)]
pub(crate) struct EmailDb {
    pub id: i64,
    pub email: String,
}
#[derive(FromRow, Debug)]
pub(crate) struct UsernameDb {
    pub id: i64,
    pub username: String,
}
#[derive(FromRow, Debug)]
pub(crate) struct UserDb {
    pub id: Uuid,
    pub email: i64,
    pub username: i64,
    pub password: i64,
    pub salt: i64,
    pub created_at: DateTime<LocalTime>,
    pub firstname: String,
    pub lastname: String,
}
