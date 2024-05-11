#![allow(unused_imports, unused_variables, dead_code)]
use core::str;

use argon2::Config;
use auth::authenticate_server::{Authenticate, AuthenticateServer};
use auth::register_response::Response;
use auth::{LoginRequest, LoginResponse, RegisterRequest, RegisterResponse};
use dashmap::{DashMap, DashSet};
use rusty_paseto::generic::{
    CustomClaim, IssuerClaim, Key, Local, PasetoError, PasetoSymmetricKey, V4,
};
use rusty_paseto::prelude::PasetoBuilder;
use sqlx::{PgPool, Postgres};
use tonic::async_trait;
use tonic::{transport::Server, Request, Response as TonicResponse, Status};
use uuid::{timestamp, Timestamp, Uuid};
pub mod auth {
    tonic::include_proto!("authentication");
}
// TODO: Generate key in a secure manner
const SUPER_SECRET_KEY: &[u8] = b"bljsalsjflfjhbaxyissocoollolhaha";
#[derive(Debug)]
struct User {
    email: String,
    username: String,
    //hashed version of password
    password: String,
    user_id: Uuid,
    salt: Uuid,
}
pub(crate) struct Authenticator {
    cockroach: PgPool,
    db: dashmap::DashMap<String, User>,
    emails: DashSet<String>,
    key: PasetoSymmetricKey<V4, Local>,
}

#[async_trait]
impl Authenticate for Authenticator {
    // FIX: make this feature with live database
    async fn register(
        &self,
        request: Request<RegisterRequest>,
    ) -> Result<TonicResponse<RegisterResponse>, Status> {
        //TODO: add logic to check if username n stuff is valid
        let cred = request.into_inner();
        let username = cred.username.clone();
        let email = cred.email.clone();
        let _register_info = self.check_register_info(&cred).await?;
        //INFO: make new user with provided and verified info
        let user = Self::new_user(cred).await?;
        let token = self.new_token(&username, &user.user_id, &email);
        self.db.insert(username, user);
        println!("token: {:?}", token);
        let response = Response::Token(token.unwrap());
        let response = Some(response);
        let register_response = RegisterResponse { response };
        Ok(TonicResponse::new(register_response))
    }
    async fn login(
        &self,
        request: Request<LoginRequest>,
    ) -> Result<TonicResponse<LoginResponse>, Status> {
        let creds = request.into_inner();
        if self.check_login_info(&creds).await? {
            //INFO: we can allow an "unwrap" here because the credentials have already been checked
            //so we know the user does infact exist
            let user = self.db.get(&creds.username).unwrap();
            let token = self.new_token(&creds.username, &user.user_id, &user.email);
            if let Err(err) = token {
                return Err(Status::internal(err.to_string()));
            }
            // let login_res = LoginResponse::;
            todo!()
        } else {
            todo!()
        }
    }
}

impl Authenticator {
    async fn check_login_info(&self, creds: &LoginRequest) -> Result<bool, Status> {
        let user = self.db.get(&creds.username);
        if let Some(user) = user {
            self.check_username(&creds.username, &user.username)?;
            self.check_password(user.password.as_str(), &creds.password)?;
            return Ok(true);
        }
        Err(Status::invalid_argument(
            "Username or Password is incorrect, try again",
        ))
    }
    fn check_username(&self, username: &str, username_attempt: &str) -> Result<(), Status> {
        if username == username_attempt {
            return Ok(());
        }
        Err(Status::invalid_argument(
            "Username or Password is incorrect, try again",
        ))
    }
    fn check_password(&self, pwd: &str, pwd_attempt: &str) -> Result<(), Status> {
        let res = argon2::verify_encoded(pwd, pwd_attempt.as_bytes());
        if let Ok(res) = res {
            return Ok(());
        }
        Err(Status::invalid_argument(
            "Username or Password is invalid, try again",
        ))
    }

    async fn check_register_info(&self, creds: &RegisterRequest) -> Result<bool, Status> {
        if self.username_available(&creds.username).await {
            return Err(Status::invalid_argument(
                "Username is already taken, try something different bozo",
            ));
        }

        if self.email_available(&creds.email).await {
            return Err(Status::invalid_argument(
                "Email is already in use, try something else bozo",
            ));
        }
        Ok(true)
    }
    pub(crate) fn new(pool: PgPool) -> Self {
        Self {
            cockroach: pool,
            db: DashMap::new(),
            emails: DashSet::new(),
            key: PasetoSymmetricKey::from(Key::from(SUPER_SECRET_KEY)),
        }
    }
    fn new_token(
        &self,
        username: &str,
        user_id: &Uuid,
        email: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        // TODO: find a way to Stringify a Uuid for it to be used in the Paseto token
        let id = user_id.clone().to_string();

        let id_string = std::str::from_utf8(id.as_bytes()).unwrap();

        // TODO: error handling

        let token = PasetoBuilder::<V4, Local>::default()
            .set_claim(IssuerClaim::from("Baxy:crabby-auth"))
            .set_claim(CustomClaim::try_from(
                ("username ".to_owned() + username).as_str(),
            )?)
            .set_claim(CustomClaim::try_from(
                ("id ".to_owned() + id_string).as_str(),
            )?)
            .set_claim(CustomClaim::try_from(
                ("email ".to_owned() + email).as_str(),
            )?)
            .build(&self.key)
            .unwrap();
        Ok(token)
    }
    async fn username_available(&self, username: &str) -> bool {
        println!("username: {:?}", username);
        println!("{:?}", self.db.contains_key(username));
        self.db.contains_key(username)
    }
    async fn email_available(&self, email: &str) -> bool {
        self.emails.contains(email)
    }
    async fn new_user(creds: RegisterRequest) -> Result<User, Status> {
        // TODO: proper error handling, im just lazy rn
        let (hash, salt) = Self::hash_password(creds.password).await?;
        let id = Self::generate_id();
        Ok(User {
            email: creds.email,
            username: creds.username,
            user_id: id,
            password: hash,
            salt,
        })
    }
    async fn hash_password(pwd: String) -> Result<(String, Uuid), Status> {
        let config = Config::default();
        let salt = Self::generate_salt();

        let hash = argon2::hash_encoded(pwd.as_bytes(), salt.as_bytes(), &config);
        if let Ok(hash) = hash {
            Ok((hash, salt))
        } else {
            Err(Status::invalid_argument(
                hash.expect_err("literally impossible").to_string(),
            ))
        }
    }
    fn generate_salt() -> Uuid {
        Uuid::new_v4()
    }
    fn generate_id() -> Uuid {
        Uuid::new_v7(Timestamp::now(timestamp::context::NoContext))
    }
    fn bootleg_into(error: PasetoError) -> Status {
        Status::internal(error.to_string())
    }
}
