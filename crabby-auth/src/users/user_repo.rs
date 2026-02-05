use std::sync::Arc;

use crate::domain::models::{RegisterRequestData, RegisterResponseData};
use eyre::Result;
use sqlx::{PgPool, query_as};
use uuid::Uuid;

use crate::domain::models::UserRow;

pub trait UserRepo {
    // add code here
    async fn register_user(&self, user: RegisterRequestData) -> Result<RegisterResponseData>;
    async fn get_user_from_id(&self, id: Uuid) -> Result<UserRow>;
    async fn get_user_from_username(&self, username: &str) -> Result<UserRow>;
}

pub struct PostgresUserRepo {
    pub conn: Arc<PgPool>,
}

impl UserRepo for PostgresUserRepo {
    async fn register_user(&self, user: RegisterRequestData) -> Result<RegisterResponseData> {
        let mut tx = self.conn.begin().await?;
        let response= query_as!(
            RegisterResponseData,
            "INSERT INTO validation.auth_user (email, username, password_hash) VALUES ($1,$2, $3) RETURNING user_id, username ",
            user.email.email.as_str(),
            user.username.username.as_str(),
            user.password.password.as_str()
        )
        .fetch_one(&mut *tx).await?;
        let _ = tx.commit().await;
        Ok(response)
    }

    async fn get_user_from_id(&self, id: Uuid) -> Result<UserRow> {
        let mut tx = self.conn.begin().await?;
        let user = query_as!(
            UserRow,
            "SELECT * from validation.auth_user where user_id = ($1)",
            id
        )
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(user)
    }

    async fn get_user_from_username(&self, username: &str) -> Result<UserRow> {
        let user = query_as!(
            UserRow,
            "SELECT * from validation.auth_user where username= ($1)",
            username
        )
        .fetch_one(&*self.conn)
        .await?;
        Ok(user)
    }
}
