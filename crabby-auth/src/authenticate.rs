#![allow(unused_imports, unused_variables, dead_code)]
use crate::{
    ARGON2, authenticate,
    domain::models::{Password, RegisterRequestData, UserRow},
    paseto::{
        self,
        keys_repo::{PasetoKeyRepo, PostgresKeyRepo},
        token::{self, UserTokens},
    },
    users::user_repo::{PostgresUserRepo, UserRepo},
};
use argon2::{
    Argon2,
    password_hash::{
        self, PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng,
    },
};
use auth::authenticate_server::{Authenticate, AuthenticateServer};
use auth::{
    LoginRequest, LoginResponse, LoginSuccess, PublicKeyRequest, PublicKeyResponse, RefreshRequest,
    RefreshResponse, RegisterRequest, RegisterResponse, RegisterSuccess,
};
use blake3::Hasher;
use chrono::Utc;
use core::str;
use dotenvy::{dotenv, var};
use eyre::{Error, Result as AnyResult};
use pasetors::{
    keys::{AsymmetricKeyPair, AsymmetricSecretKey, Generate, SymmetricKey},
    paserk::{FormatAsPaserk, Id},
    version4::V4,
};
use sqlx::prelude::FromRow;
use sqlx::types::chrono::{DateTime, Local as LocalTime};
use sqlx::{Acquire, PgPool, Postgres, query, query_as};
use std::{
    marker::PhantomData,
    str::FromStr,
    sync::{Arc, LazyLock},
};
use tonic::{Code, async_trait};
use tonic::{Request, Response as TonicResponse, Status, transport::Server};
use tracing_subscriber::fmt::format;
use uuid::{Timestamp, Uuid, timestamp};
use validator::Validate;

pub mod auth {
    tonic::include_proto!("authentication");
}

#[derive(Debug)]
struct UserMetadataForToken {
    username: String,
    id: Uuid,
}
pub(crate) struct Authenticator<U, K>
where
    U: UserRepo + Send + Sync,
    K: PasetoKeyRepo + Send + Sync,
{
    user_repo: U,
    keys_repo: K,
    symmetric_key: SymmetricKey<V4>,
    asymmetric_kp: AsymmetricKeyPair<V4>,
}

#[async_trait]
impl Authenticate for Authenticator<PostgresUserRepo, PostgresKeyRepo> {
    async fn register(
        &self,
        request: Request<RegisterRequest>,
    ) -> Result<TonicResponse<RegisterResponse>, Status> {
        //INFO: make new user with provided and verified info
        //
        //
        let inner = request.into_inner();
        let mut data = RegisterRequestData::new(inner);
        data.validate()
            //TODO: be more verbose, what exactly failed
            .map_err(|e| Status::invalid_argument("registration data invalid"))?;
        data.password = Self::hash_password(data.password).await?;
        let user = self
            .user_repo
            .register_user(data)
            .await
            .map_err(|e| Status::invalid_argument("Failed to register"))?;

        let tokens = self.generate_tokens(&user.username, &user.user_id)?;
        let refresh_token = tokens.refresh.token();
        let possible_error = self.keys_repo.store_refresh_info(&tokens.refresh).await;
        if let Err(possible_error) = possible_error {
            // FIX: unsafe to return database error
            let err = format!("error: {:?}", possible_error);
            // let _ = query!("ROLLBACK;").execute(&self.db).await;
            return Err(Status::cancelled(err));
        }

        let response = RegisterSuccess {
            bearer: tokens.bearer,
            refresh: refresh_token,
            username: user.username,
            user_id: user.user_id.hyphenated().to_string(),
        };
        let response = Some(response);
        let register_response = RegisterResponse { response };
        Ok(TonicResponse::new(register_response))
    }

    async fn login(
        &self,
        request: Request<LoginRequest>,
    ) -> Result<TonicResponse<LoginResponse>, Status> {
        self.login(request).await
    }

    async fn refresh(
        &self,
        refresh: Request<RefreshRequest>,
    ) -> Result<TonicResponse<RefreshResponse>, Status> {
        let metadata = refresh.metadata();
        let creds = refresh.into_inner();

        let user_id =
            Uuid::from_str(&creds.user_id).map_err(|err| Status::internal("internal error"))?;
        todo!()
    }
    //INFO: this function needs to be changed in the future once key rotation has been implemented
    //to implement appropriate database queries related to making sure the paserk ID is a valid one.
    async fn public_key(
        &self,
        key: tonic::Request<PublicKeyRequest>,
    ) -> Result<TonicResponse<PublicKeyResponse>, Status> {
        //
        let request_info = key.into_inner().req;

        let public_key = &self.asymmetric_kp.public;
        let paserk_id = Id::from(public_key);
        let attempted_id = Id::try_from(request_info.as_str())
            .map_err(|err| Status::permission_denied("Not signed by crabby-chatty"))?;

        //INFO: This is just a decoy check to say that it was checked
        if paserk_id == attempted_id {
            let mut paserk_response = String::new();
            public_key
                .fmt(&mut paserk_response)
                .map_err(|err| Status::internal("Internal failure"))?;
            return Ok(TonicResponse::new(PublicKeyResponse {
                paserk: paserk_response,
            }));
        } else {
            return Err(Status::unauthenticated("not a valid PID"));
        }
    }
}
//
impl Authenticator<PostgresUserRepo, PostgresKeyRepo> {
    pub fn new(pool: PgPool) -> Self {
        let super_secret_key = var("SUPER_SECRET_KEY").expect("Set super secret key");
        let paseto_key = SymmetricKey::<V4>::generate().unwrap();
        let connection_pool = Arc::new(pool);
        Self {
            keys_repo: PostgresKeyRepo {
                conn: connection_pool.clone(),
            },
            user_repo: PostgresUserRepo {
                conn: connection_pool.clone(),
            },
            symmetric_key: paseto_key,
            asymmetric_kp: AsymmetricKeyPair::generate().unwrap(),
        }
    }
    //
    fn generate_tokens(&self, username: &str, user_id: &Uuid) -> Result<UserTokens, Status> {
        //FIX: find a better way to generate unique IDs
        let mut buffer = Uuid::encode_buffer();
        let id = user_id.as_hyphenated().encode_lower(&mut buffer);

        let bearer = token::bearer(username, id, &self.asymmetric_kp)?;
        let refresh = token::refresh(&self.symmetric_key, id)?;
        //FIX: Should insertion of refresh token metadata be done inside this function?
        Ok(UserTokens::new(bearer, refresh))
    }

    async fn hash_password(pwd: Password) -> Result<Password, Status> {
        let salt = &SaltString::generate(&mut OsRng);
        let pass = pwd.password;
        let hash = ARGON2.hash_password(pass.as_bytes(), salt);
        if let Ok(hashed) = hash {
            let hashed_password = Password {
                password: hashed.to_string(),
            };
            Ok(hashed_password)
        } else {
            Err(Status::invalid_argument("wrong password or username"))
        }
    }

    //
    async fn login(
        &self,
        request: Request<LoginRequest>,
    ) -> Result<TonicResponse<LoginResponse>, Status> {
        let metadata = request.metadata();
        let creds = request.into_inner();
        let user = self
            .user_repo
            .get_user_from_username(&creds.username)
            .await
            .map_err(|err| Status::unauthenticated("unauthenticated request"))?;
        let password_hash =
            PasswordHash::new(&user.password_hash.password).expect("argon2 type from string");
        match ARGON2.verify_password(creds.password.as_bytes(), &password_hash) {
            Ok(_) => {
                let login_success = LoginSuccess {
                    user_id: user.user_id.hyphenated().to_string(),
                    username: creds.username.to_owned(),
                };
                Ok(TonicResponse::new(LoginResponse {
                    login_success: Some(login_success),
                }))
            }
            _ => Err(Status::unauthenticated("Could not authenticate")),
        }
    }
    //
}
