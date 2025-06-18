use sqlx::prelude::FromRow;
use sqlx::types::chrono::{DateTime, Utc};
use uuid::Uuid;
#[derive(FromRow, Debug)]
struct RowId(i64);
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
    pub user_id: Uuid,
    pub email_id: i64,
    pub username_id: i64,
    pub password_id: i64,
    pub created_at: DateTime<Utc>,
    pub last_login_id: i64,
    pub is_admin: bool,
}
#[derive(FromRow, Debug)]
pub(crate) struct IpDb {
    id: i64,
    ip: String,
}
#[derive(Debug, FromRow)]
struct BearerDb {
    id: i64,
    bearer: String,
}
#[derive(Debug, FromRow)]
struct RefreshDb {
    id: i64,
    refresh: String,
}
#[derive(Debug, FromRow)]
struct TokensDb {
    id: i64,
    bearer: i64,
    refresh: i64,
}
