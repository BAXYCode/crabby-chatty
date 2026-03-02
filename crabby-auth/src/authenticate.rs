#![allow(unused_imports, unused_variables, dead_code)]
use crate::{
    ARGON2,
    authenticate::{self, auth::RefreshSuccess},
    domain::models::{
        ConvertToken, Password, RefreshTokenRow, RefreshTokenWithMetadata, RegisterRequestData,
        UserRow,
    },
    intercept::TokenExtension,
    paseto::{
        self,
        claims_config::ClaimsConfig,
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
use chrono::{Duration, Utc};
use core::str;
use crabby_core::tokens::{KeyRetrieval, VerifyToken};
use dotenvy::{dotenv, var};
use eyre::{Error, Result as AnyResult};
use hmac::{Hmac, Mac};
use pasetors::{
    claims::ClaimsValidationRules,
    footer::Footer,
    keys::{AsymmetricKeyPair, AsymmetricPublicKey, AsymmetricSecretKey, Generate, SymmetricKey},
    local::decrypt,
    paserk::{FormatAsPaserk, Id},
    token::UntrustedToken,
    version4::V4,
};
use sha2::Sha256;
use sqlx::prelude::FromRow;
use sqlx::types::chrono::{DateTime, Local as LocalTime};
use sqlx::{Acquire, PgPool, Postgres, query, query_as};
use std::{
    marker::PhantomData,
    str::FromStr,
    sync::{Arc, LazyLock},
};
use tokio::task;
use tonic::{Code, async_trait};
use tonic::{Request, Response as TonicResponse, Status, transport::Server};
use tracing_subscriber::fmt::format;
use uuid::{Timestamp, Uuid, timestamp};
use validator::Validate;

pub mod auth {
    tonic::include_proto!("authentication");
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
    pepper: String,
    claims_config: ClaimsConfig,
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
        let refresh_token = tokens.refresh;
        let token_hash = self
            .hash_refresh_token(refresh_token.token.as_str())
            .map_err(|e| Status::internal("hashing issue"))?;

        let refresh_token_row = refresh_token.to_row(token_hash);
        let possible_error = self.keys_repo.store_refresh_info(&refresh_token_row).await;
        if let Err(possible_error) = possible_error {
            // FIX: unsafe to return database error
            let err = format!("error: {:?}", possible_error);
            // let _ = query!("ROLLBACK;").execute(&self.db).await;
            return Err(Status::cancelled(err));
        }

        let response = RegisterSuccess {
            bearer: tokens.bearer,
            refresh: refresh_token.token,
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
        mut refresh: Request<RefreshRequest>,
    ) -> Result<TonicResponse<RefreshResponse>, Status> {
        let token_extension = refresh.extensions_mut().remove::<TokenExtension>();

        // let user_id =
        //     Uuid::from_str(&creds.user_id).map_err(|err| Status::internal("internal error"))?;
        //     Verify token was extracted from Authorization header
        if let Some(token_extension) = token_extension {
            let token_string = token_extension.into_inner();
            //TODO: NEED WAY BETTER ERROR HANDLING this map_err is redundant as I map it in the
            //verify method, but I can fix this later
            let trusted_token = self
                .verify(token_string.clone())
                .await
                .map_err(|e| Status::unauthenticated("unauthenticated api call attempt"))?;
            let claims = trusted_token
                .payload_claims()
                .ok_or(Status::unauthenticated("UNAUTHENTICATED REQUEST claims"))?;
            if let Some(user_id) = claims.get_claim("sub")
                && let Some(user_id) = user_id.as_str()
            {
                //verify the refresh token compared with what we have stored in DB
                let jti = claims
                    .get_claim("jti")
                    .ok_or(Status::unauthenticated("UNAUTHENTICATED REQUEST jti"))?
                    .as_str()
                    .ok_or(Status::unauthenticated("UNAUTHENTICATED REQUEST jti2"))?;
                let parsed_jti = Uuid::parse_str(jti)
                    .map_err(|e| Status::unauthenticated("UNAUTHENTICATED REQUEST parse jti"))?;
                let parsed_user_id = Uuid::parse_str(user_id).map_err(|e| {
                    Status::unauthenticated("UNAUTHENTICATED REQUEST parse user id")
                })?;
                let stored_token_info = self
                    .keys_repo
                    .fetch_refresh_token(&parsed_jti, &parsed_user_id)
                    .await
                    .map_err(|e| {
                        Status::unauthenticated("UNAUTHENTICATED REQUEST fetch refresh")
                    })?;
                //Error out if hashes don't match
                self.verify_refresh_hash(token_string, &stored_token_info.token_hash)
                    .map_err(|e| {
                        Status::unauthenticated("UNAUTHENTICATED REQUEST verify refresh")
                    })?;

                let user = self
                    .user_repo
                    .get_user_from_id(parsed_user_id)
                    .await
                    .map_err(|e| Status::unauthenticated("UNAUTHENTICATED get user"))?;

                let tokens = self
                    .generate_tokens(user.username.username.as_str(), &user.user_id)
                    .map_err(|e| Status::internal("whoops"))?;
                let new_access_token = tokens.bearer;
                let now = Utc::now();
                let leeway = Duration::hours(72);
                let should_rotate = stored_token_info.expires_at - now <= leeway;

                if !should_rotate {
                    return Ok(TonicResponse::new(RefreshResponse {
                        refresh: Some(RefreshSuccess {
                            bearer: new_access_token,
                            refresh: None,
                        }),
                    }));
                }
                let new_refresh_token = tokens.refresh.token.clone();
                let new_refresh_token_hash = self
                    .hash_refresh_token(new_refresh_token.as_str())
                    .map_err(|e| Status::internal("whoops"))?;
                //Store new refresh token info and send out both refresh and access tokens in
                //respone
                let new_refresh_stored_info = tokens.refresh.to_row(new_refresh_token_hash);

                let rotated = self
                    .keys_repo
                    .rotate_refresh_info(
                        &stored_token_info.user_id,
                        &stored_token_info.token_jti,
                        &stored_token_info.token_hash,
                        &new_refresh_stored_info,
                    )
                    .await
                    .map_err(|e| Status::unauthenticated("UNAUTHENTICATED rotated"))?;

                if !rotated {
                    return Err(Status::unauthenticated(
                        "UNAUTHENTICATED REQUEST not retated1",
                    ));
                }

                return Ok(TonicResponse::new(RefreshResponse {
                    refresh: Some(RefreshSuccess {
                        bearer: new_access_token,
                        refresh: Some(new_refresh_token),
                    }),
                }));
            }
        }
        Err(Status::unauthenticated("UNAUTHENTICATED REQUEST big boy"))
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
            pepper: super_secret_key,
            claims_config: ClaimsConfig::new(),
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
        match Self::verify_password(creds.password, user.password_hash.password).await {
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
    async fn verify_password(password: String, hash_str: String) -> Result<(), Status> {
        task::spawn_blocking(move || {
            let parsed = PasswordHash::new(&hash_str)
                .map_err(|_| Status::internal("stored hash invalid"))?;

            ARGON2
                .verify_password(password.as_bytes(), &parsed)
                .map_err(|_| Status::unauthenticated("bad credentials"))?;

            Ok(())
        })
        .await
        .map_err(|_| Status::internal("verify task failed"))?
    }
}
impl<U, K> Authenticator<U, K>
where
    U: UserRepo + Send + Sync,
    K: PasetoKeyRepo + Send + Sync,
{
    fn verify_refresh_hash(&self, claimed_token: String, stored_hash: &[u8]) -> AnyResult<()> {
        type HmacSha256 = Hmac<Sha256>;

        let mut hasher = HmacSha256::new_from_slice(self.pepper.as_bytes()).expect("hasher");
        hasher.update(claimed_token.as_bytes());
        hasher.verify_slice(stored_hash)?;
        Ok(())
    }
    fn hash_refresh_token(&self, token: &str) -> AnyResult<Vec<u8>> {
        type HmacSha256 = Hmac<Sha256>;

        let mut hasher = HmacSha256::new_from_slice(self.pepper.as_bytes()).expect("hasher");
        hasher.update(token.as_bytes());
        let hash = hasher.finalize();
        let mut result = Vec::new();
        result.extend_from_slice(hash.into_bytes().as_slice());
        Ok(result)
    }
    async fn hash_password(pwd: Password) -> Result<Password, Status> {
        task::spawn_blocking(move || {
            let salt = SaltString::generate(&mut OsRng);
            let hash = ARGON2
                .hash_password(pwd.password.as_bytes(), &salt)
                .map_err(|_| Status::internal("password hashing failed"))?
                .to_string();

            Ok(Password { password: hash })
        })
        .await
        .map_err(|_| Status::internal("hash task failed"))?
    }
    fn generate_tokens(&self, username: &str, user_id: &Uuid) -> Result<UserTokens, Status> {
        //FIX: find a better way to generate unique IDs
        let mut buffer = Uuid::encode_buffer();
        let id = user_id.as_hyphenated().encode_lower(&mut buffer);

        let bearer = token::bearer(username, id, &self.asymmetric_kp)?;
        let refresh = token::refresh(&self.symmetric_key, id)?;
        //FIX: Should insertion of refresh token metadata be done inside this function?
        Ok(UserTokens::new(bearer, refresh))
    }
}

impl<U, K> VerifyToken<SymmetricKey<V4>> for Authenticator<U, K>
where
    U: UserRepo + Send + Sync,
    K: PasetoKeyRepo + Send + Sync,
    K: KeyRetrieval<SymmetricKey<V4>>,
{
    type Storage = K;

    async fn verify(&self, token: String) -> AnyResult<pasetors::token::TrustedToken> {
        let current_key_id = Id::from(&self.symmetric_key);
        let untrusted = UntrustedToken::try_from(token.as_str())
            .map_err(|e| Status::unauthenticated("unauthenticated request"))?;

        let mut untrusted_footer = Footer::new();
        untrusted_footer
            .parse_bytes(untrusted.untrusted_footer())
            .map_err(|e| Status::unauthenticated("unauthenticated request"))?;
        //if the kid in the footer isn't the kid of the currently active key
        if let Some(value) = untrusted_footer.get_claim("kid")
            && let Some(string) = value.as_str()
            && Id::try_from(string).map_err(|e| {
                eyre::Report::new(Status::unauthenticated("unauthenticated api call attempt"))
            })? != current_key_id
        {
            let key = self.keys_repo.fetch_local_key(string.to_string()).await?;

            let token = decrypt(
                &key,
                &untrusted,
                self.claims_config.refresh(),
                Some(&untrusted_footer),
                None,
            )?;
            return Ok(token);
        }
        if let Ok(token) = decrypt(
            &self.symmetric_key,
            &untrusted,
            self.claims_config.refresh(),
            Some(&untrusted_footer),
            None,
        ) {
            return Ok(token);
        }

        Err(eyre::Report::new(Status::unauthenticated(
            "unauthenticated api call attempt",
        )))
    }
}
