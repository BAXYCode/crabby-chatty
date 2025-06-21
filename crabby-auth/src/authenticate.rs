#![allow(unused_imports, unused_variables, dead_code)]
use crate::{
    model::{BearerRow, RefreshRow, TokensRow},
    token::{self, UserTokens},
    ARGON2,
};
use anyhow::{Error, Result as AnyResult};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use auth::authenticate_server::{Authenticate, AuthenticateServer};
use auth::register_response::Response;
use auth::{
    AuthRequest, AuthResponse, LoginRequest, LoginResponse, RefreshRequest, RefreshResponse,
    RegisterRequest, RegisterResponse, RegisterSuccess,
};
use chrono::Utc;
use core::str;
use dashmap::{DashMap, DashSet};
use dotenvy::{dotenv, var};
use pasetors::{keys::SymmetricKey, version4::V4};
use sqlx::prelude::FromRow;
use sqlx::types::chrono::{DateTime, Local as LocalTime};
use sqlx::{query, query_as, Acquire, PgPool, Postgres};
use std::sync::LazyLock;
use tonic::{async_trait, Code};
use tonic::{transport::Server, Request, Response as TonicResponse, Status};
use tracing_subscriber::fmt::format;
use uuid::{timestamp, Timestamp, Uuid};

use crate::model::{EmailRow, PasswordRow, UserRow, UsernameRow};

use self::auth::login_response::Login;
pub mod auth {
    tonic::include_proto!("authentication");
}

#[derive(Debug)]
struct User {
    username: String,
    id: Uuid,
}
pub(crate) struct Authenticator {
    db: PgPool,
    paseto_key: SymmetricKey<V4>,
}

#[async_trait]
impl Authenticate for Authenticator {
    // FIX: make this feature with live database
    async fn register(
        &self,
        request: Request<RegisterRequest>,
    ) -> Result<TonicResponse<RegisterResponse>, Status> {
        //INFO: make new user with provided and verified info
        let user = self.register(request).await;
        if let Err(user) = user {
            // FIX: unsafe to return database error
            let err = format!("error: {:?}", user);
            return Err(Status::invalid_argument(err));
        }
        //INFO: unwrap is fine here because err is handled above
        // println!("user: {:?} has successfully registered", user);
        let user = user.unwrap();
        let tokens = self.generate_tokens(&user.username, &user.id)?;
        let possible_error = self.insert_tokens(&tokens, &user.id).await;
        if let Err(possible_error) = possible_error {
            // FIX: unsafe to return database error
            let err = format!("error: {:?}", possible_error);
            let _ = query!("ROLLBACK;").execute(&self.db).await;
            return Err(Status::cancelled(err));
        }

        let response = Response::Response(RegisterSuccess {
            bearer: tokens.bearer,
            refresh: tokens.refresh,
            username: user.username,
            user_id: user.id.hyphenated().to_string(),
        });
        let response = Some(response);
        let register_response = RegisterResponse { response };
        Ok(TonicResponse::new(register_response))
    }
    async fn login(
        &self,
        request: Request<LoginRequest>,
    ) -> Result<TonicResponse<LoginResponse>, Status> {
        let creds = request.into_inner();
        todo!()
    }
    async fn refresh(
        &self,
        refresh: Request<RefreshRequest>,
    ) -> Result<TonicResponse<RefreshResponse>, Status> {
        todo!();
    }
    async fn authenticate(
        &self,
        authenticate: Request<AuthRequest>,
    ) -> Result<TonicResponse<AuthResponse>, Status> {
        todo!();
    }
}

impl Authenticator {
    async fn get_email_from_id(&self, id: i64) -> Result<EmailRow, Status> {
        let email_row = query_as!(
            EmailRow,
            "select id, email from valid.email where id=($1);",
            id
        )
        .fetch_one(&self.db)
        .await
        .map_err(|err| {
            let err = format!("sqlx error: {:?}", err);
            Status::internal(err)
        })?;
        Ok(email_row)
    }
    async fn get_user_from_username(&self, username: &str) -> Result<UserRow, Status> {
        let user = self
            .get_user_row_from_username_id(self.get_username_row_from_username(username).await?.id)
            .await?;
        Ok(user)
    }
    async fn get_username_row_from_username(&self, username: &str) -> Result<UsernameRow, Status> {
        let username_row: UsernameRow = query_as!(
            UsernameRow,
            "select id, username from valid.username where username=($1);",
            username
        )
        .fetch_one(&self.db)
        .await
        .map_err(|err| {
            let err = format!("sqlx error: {:?}", err);
            Status::internal(err)
        })?;
        Ok(username_row)
    }
    async fn get_user_row_from_username_id(&self, id: i64) -> Result<UserRow, Status> {
        let user = query_as!(
            UserRow,
           r#"select user_id, email_id, username_id, password_id, created_at as "created_at: DateTime<Utc>", last_login_id, is_admin  from valid.users where username_id=($1)"#,
            id
        )
        .fetch_one(&self.db)
        .await
        .map_err(|err| {
            let err = format!("sqlx error: {:?}", err);
            Status::internal(err)
        })?;
        Ok(user)
    }
    async fn get_password_from_id(&self, id: i64) -> Result<PasswordRow, Status> {
        let password = query_as!(
            PasswordRow,
            "select * from valid.password where id=($1)",
            id
        )
        .fetch_one(&self.db)
        .await
        .map_err(|err| {
            let err = format!("sqlx error: {:?}", err);
            Status::internal(err)
        })?;
        Ok(password)
    }
    async fn check_username(&self, username: &str) -> Result<(), Status> {
        // FIX: error handling
        let con = self.db.acquire().await.unwrap();
        let username_available = query!(
            "select * from valid.username where username=($1);",
            username
        )
        .fetch_optional(&self.db)
        .await
        .map_err(|err| Status::internal("sqlx is fucked"))?;
        if let Some(username_db) = username_available {
            return Err(Status::already_exists("Username already exists"));
        }
        Ok(())
    }
    pub(crate) fn new(pool: PgPool) -> Self {
        let super_secret_key = var("SUPER_SECRET_KEY").expect("Set super secret key");
        let paseto_key = SymmetricKey::<V4>::from(super_secret_key.as_bytes()).unwrap();
        Self {
            db: pool,
            paseto_key: SymmetricKey::from(super_secret_key.as_bytes())
                .expect("Paseto symmetric key"),
        }
    }
    fn generate_tokens(&self, username: &str, user_id: &Uuid) -> AnyResult<UserTokens, Status> {
        let mut buffer = Uuid::encode_buffer();
        let id = user_id.as_hyphenated().encode_lower(&mut buffer);
        let bearer = token::bearer(username, id, false, self.as_ref())?;
        let refresh = token::refresh(self.as_ref(), 1)?;
        Ok(UserTokens::new(bearer, refresh))
    }
    async fn email_available(&self, email: &str) -> bool {
        let email: Result<EmailRow, sqlx::Error> =
            query_as(format!("SELECT * FROM valid.email WHERE email = {}", email).as_str())
                .fetch_one(&self.db)
                .await;
        if let Err(err) = email {
            return true;
        }
        false
    }
    async fn hash_password(pwd: &str) -> Result<String, Status> {
        let salt = &SaltString::generate(&mut OsRng);
        let hash = ARGON2.hash_password(pwd.as_bytes(), salt);
        if let Ok(hashed) = hash {
            Ok(hashed.to_string())
        } else {
            Err(Status::invalid_argument("wrong password or username"))
        }
    }
    // FIX: unsafe error handling, exposing internal schema
    async fn register(&self, req: Request<RegisterRequest>) -> AnyResult<User> {
        let creds = req.into_inner();
        let transaction = self.db.begin().await?;

        let password_hash = Self::hash_password(&creds.password).await?;
        let username = creds.username.clone();
        let email = creds.email.clone();

        // .map_err(|err| Error::new(err).context("salt issue"))?;
        let password: PasswordRow = query_as!(
            PasswordRow,
            "INSERT INTO valid.password (password) VALUES ($1) RETURNING *;",
            password_hash
        )
        .fetch_one(&self.db)
        .await?;
        let username: UsernameRow = query_as!(
            UsernameRow,
            " INSERT INTO valid.username (username) VALUES ($1) RETURNING *; ",
            username
        )
        .fetch_one(&self.db)
        .await?;
        let email: EmailRow = query_as!(
            EmailRow,
            "INSERT INTO valid.email (email) VALUES ( $1 ) RETURNING *;",
            email
        )
        .fetch_one(&self.db)
        .await
        .map_err(|err| Error::new(err).context("email issue"))?;
        let user: UserRow = query_as!(UserRow,
                "INSERT INTO valid.users ( email_id, username_id, password_id) VALUES ($1, $2, $3) RETURNING *;",
                email.id,
                username.id,
                password.id,
        )
        .fetch_one(&self.db)
        .await.map_err(|err| Error::new(err).context("User table issue"))?;
        transaction
            .commit()
            .await
            .map_err(|err| Error::new(err).context("transaction issue"))?;

        Ok(User {
            id: user.user_id,
            username: username.username,
        })
    }

    async fn insert_tokens(&self, tokens: &UserTokens, user_id: &Uuid) -> AnyResult<()> {
        let transaction = self.db.begin().await?;
        let bearer: BearerRow = query_as!(
            BearerRow,
            "INSERT INTO valid.bearer (bearer) VALUES ($1) RETURNING *;",
            tokens.bearer
        )
        .fetch_one(&self.db)
        .await
        .map_err(|err| Error::new(err).context("transaction issue"))?;
        let refresh: RefreshRow = query_as!(
            RefreshRow,
            "INSERT INTO valid.refresh (refresh) VALUES ($1) RETURNING *;",
            tokens.refresh
        )
        .fetch_one(&self.db)
        .await
        .map_err(|err| Error::new(err).context("transaction issue"))?;
        let _ = query!(
            "INSERT INTO valid.tokens (user_id,bearer_id, refresh_id) VALUES ($1, $2, $3) ;",
            user_id,
            bearer.id,
            refresh.id
        )
        .execute(&self.db)
        .await
        .map_err(|err| Error::new(err).context("transaction issue"))?;

        transaction
            .commit()
            .await
            .map_err(|err| Error::new(err).context("transaction issue"))?;
        Ok(())
    }
}

impl AsRef<SymmetricKey<V4>> for Authenticator {
    fn as_ref(&self) -> &SymmetricKey<V4> {
        &self.paseto_key
    }
}
