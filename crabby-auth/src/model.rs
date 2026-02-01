use crate::token::UserTokens;
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;
use sqlx::types::chrono::{DateTime, Utc};
use uuid::Uuid;
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
#[derive(Serialize, Deserialize, FromRow)]
pub(crate) struct RefreshMetadataRow {
    #[sqlx(rename = "userId")]
    /*rust convention would expect the field to be named as written under, but sqlx macros won't
    wok with `FromRow`*/
    //pub userid: Uuid,
    pub userid: Uuid,
    pub id: Uuid,
    pub token_hash: String,
    pub iat: DateTime<Utc>,
    pub nbf: DateTime<Utc>,
    pub exp: DateTime<Utc>,
}
#[derive(Debug, FromRow)]
pub(crate) struct TokensRow {
    pub id: Uuid,
    pub bearer: String,
    pub refresh: String,
}

// impl From<TokensRow> for UserTokens {
//     fn from(value: TokensRow) -> Self {
//         UserTokens::new(value.bearer, value.refresh)
//     }
// }
