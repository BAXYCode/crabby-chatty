use crate::authenticate::auth::RegisterRequest;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use sqlx::prelude::FromRow;
use uuid::Uuid;
use validator::Validate;
// pub mod auth {
//     tonic::include_proto!("authentication");
// }
#[derive(Clone, Validate, Deserialize, Debug)]
#[serde(transparent)]
pub struct EmailAddress {
    #[validate(email)]
    pub email: String,
}

impl From<String> for EmailAddress {
    fn from(value: String) -> Self {
        EmailAddress { email: value }
    }
}
//Newtype for username validation
#[derive(Clone, Validate, Deserialize, Debug)]
#[serde(transparent)]
pub struct Username {
    #[validate(length(min = 5, max = 20))]
    pub username: String,
}
impl From<String> for Username {
    fn from(value: String) -> Self {
        Username { username: value }
    }
}
#[derive(Clone, Validate, Deserialize, Debug)]
#[serde(transparent)]
pub struct Password {
    #[validate(length(min = 8))]
    pub password: String,
}
impl From<String> for Password {
    fn from(value: String) -> Self {
        Password { password: value }
    }
}
// impl From<Option<String>> for Password {
//     fn from(value: String) -> Self {
//         Password { password: value }
//     }
// }
// DTO for register requests
#[derive(Clone, Validate, Deserialize)]
pub struct RegisterRequestData {
    pub username: Username,
    pub email: EmailAddress,
    pub password: Password,
}
pub struct RegisterResponseData {
    pub user_id: Uuid,
    pub username: String,
}
#[derive(FromRow, Debug)]
pub(crate) struct UserRow {
    pub user_id: Uuid,
    pub email: EmailAddress,
    pub username: Username,
    pub password_hash: Password,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl RegisterRequestData {
    pub fn new(req: RegisterRequest) -> Self {
        let username = Username {
            username: req.username,
        };
        let password = Password {
            password: req.password,
        };
        let email = EmailAddress { email: req.email };
        Self {
            username,
            email,
            password,
        }
    }
}
pub struct RefreshTokenWithMetadata {
    pub token: String,
    pub user_id: Uuid,
    pub jti: Uuid,
    pub issued_at: DateTime<Utc>,
    pub exp: DateTime<Utc>,
}
impl RefreshTokenWithMetadata {
    pub fn token(&self) -> String {
        self.token.clone()
    }
}
