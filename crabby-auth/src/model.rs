use sqlx::prelude::FromRow;
use sqlx::types::chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::token::UserTokens;
#[derive(FromRow, Debug)]
struct RowId(i64);
#[derive(FromRow)]
pub(crate) struct PasswordRow {
    pub id: i64,
    pub password: String,
}
#[derive(FromRow)]
pub(crate) struct EmailRow {
    pub id: i64,
    pub email: String,
}
#[derive(FromRow, Debug)]
pub(crate) struct UsernameRow {
    pub id: i64,
    pub username: String,
}
#[derive(FromRow, Debug)]
pub(crate) struct UserRow {
    pub user_id: Uuid,
    pub email_id: i64,
    pub username_id: i64,
    pub password_id: i64,
    pub created_at: DateTime<Utc>,
    pub last_login_id: i64,
    pub is_admin: bool,
}
#[derive(FromRow, Debug)]
pub(crate) struct IpRow {
    id: i64,
    ip: String,
}
#[derive(Debug, FromRow)]
pub(crate) struct BearerRow {
    pub id: i64,
    pub bearer: String,
}
#[derive(Debug, FromRow)]
pub(crate) struct RefreshRow {
    pub id: i64,
    pub refresh: String,
    pub version: i64,
}
#[derive(Debug, FromRow)]
pub(crate) struct TokensRow {
    pub id: Uuid,
    pub bearer: String,
    pub refresh: String,
}

impl From<TokensRow> for UserTokens {
    fn from(value: TokensRow) -> Self {
        UserTokens::new(value.bearer, value.refresh)
    }
}
