#![allow(unused_imports, unused_variables, dead_code)]
use anyhow::{Error, Result as AnyResult};
use argon2::Config;
use auth::authenticate_server::{Authenticate, AuthenticateServer};
use auth::register_response::Response;
use auth::{LoginRequest, LoginResponse, RegisterRequest, RegisterResponse};
use chrono::Utc;
use core::str;
use dashmap::{DashMap, DashSet};
use rusty_paseto::generic::{
    CustomClaim, IssuerClaim, Key, Local, PasetoError, PasetoSymmetricKey, V4,
};
use rusty_paseto::prelude::PasetoBuilder;
use sqlx::prelude::FromRow;
use sqlx::types::chrono::{DateTime, Local as LocalTime};
use sqlx::{query, query_as, Acquire, PgPool, Postgres};
use tonic::{async_trait, Code};
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
    id: Uuid, //hashed version of password
}
pub(crate) struct Authenticator {
    cockroach: PgPool,
    db: dashmap::DashMap<String, User>,
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
        //
        // let cred = request.into_inner();
        // let username = cred.username.clone();
        // let email = cred.email.clone();
        // let _register_info = self.check_register_info(&cred).await?;
        // //INFO: make new user with provided and verified info
        //
        // let user = Self::new_user(cred).await?;
        let user = self.register_transaction(request).await;
        if let Err(user) = user {
            return Err(Status::invalid_argument(
                "Invalid arguments, username or email is unavailable",
            ));
        }
        let user = user.unwrap();
        let token = self.new_token(&user.username, &user.id, &user.email);
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
            let token = self.new_token(&creds.username, &user.id, &user.email);
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
        let username_row: UsernameDb = query_as!(
            UsernameDb,
            "select id, username from valid.username where username=($1);",
            &creds.username
        )
        .fetch_one(&self.cockroach)
        .await
        .map_err(|_| Status::internal("sqlx is fucked up kekW"))?;

        let user = query_as!(
            UserDb,
            "select * from valid.users where username=($1)",
            username_row.id
        )
        .fetch_one(&self.cockroach)
        .await
        .map_err(|_| Status::internal("sqlx is cooked my bad gang"));

        if let Ok(user) = user {
            self.check_username(&creds.username).await.map_err(|err| {
                if err.code() == Code::Internal {
                    Status::internal("internal error, sorry!")
                } else {
                    Status::invalid_argument("Username or password not matching our records")
                }
            })?;
            // self.check_password(user.password.as_str(), &creds.password)?;

            Ok(true)
        } else {
            Err(Status::invalid_argument(
                "Username or Password is incorrect, try again",
            ))
        }
    }
    async fn check_username(&self, username: &str) -> AnyResult<(), Status> {
        // FIX: error handling
        let con = self.cockroach.acquire().await.unwrap();
        let username_available = query!(
            "select * from valid.username where username=($1);",
            username
        )
        .fetch_optional(&self.cockroach)
        .await
        .map_err(|err| Status::internal("sqlx is fucked"))?;
        if let Some(username_db) = username_available {
            return Err(Status::already_exists("inner"));
        }
        Ok(())
    }
    // fn check_password(&self, pwd: &str, pwd_attempt: &str) -> Result<(), Status> {
    //     let res = argon2::verify_encoded(pwd, pwd_attempt.as_bytes());
    //     if let Ok(res) = res {
    //         return Ok(());
    //     }
    //     Err(Status::invalid_argument(
    //         "Username or Password is invalid, try again",
    //     ))
    // }

    // async fn check_register_info(&self, creds: &RegisterRequest) -> Result<bool, Status> {
    //     if self.username_available(&creds.username).await {
    //         return Err(Status::invalid_argument(
    //             "Username is already taken, try something different bozo",
    //         ));
    //     }
    //
    //     if self.email_available(&creds.email).await {
    //         return Err(Status::invalid_argument(
    //             "Email is already in use, try something else bozo",
    //         ));
    //     }
    //     Ok(true)
    // }
    pub(crate) fn new(pool: PgPool) -> Self {
        Self {
            cockroach: pool,
            db: DashMap::new(),
            key: PasetoSymmetricKey::from(Key::from(SUPER_SECRET_KEY)),
        }
    }
    fn new_token(&self, username: &str, user_id: &Uuid, email: &str) -> Result<String, Status> {
        // TODO: find a way to Stringify a Uuid for it to be used in the Paseto token
        let id = user_id.clone().to_string();

        let id_string = std::str::from_utf8(id.as_bytes()).unwrap();

        // TODO: error handling

        let token = PasetoBuilder::<V4, Local>::default()
            .set_claim(IssuerClaim::from("Baxy:crabby-auth"))
            .set_claim(
                CustomClaim::try_from(("username ".to_owned() + username).as_str())
                    .map_err(|err| Status::internal("internal error with Paseto token Builder"))?,
            )
            .set_claim(
                CustomClaim::try_from(("id ".to_owned() + id_string).as_str())
                    .map_err(|err| Status::internal("internal error with Paseto token Builder"))?,
            )
            .set_claim(
                CustomClaim::try_from(("email ".to_owned() + email).as_str())
                    .map_err(|err| Status::internal("internal error with Paseto token Builder"))?,
            )
            .build(&self.key)
            .map_err(|err| Status::internal("internal error with Paseto token Builder"))?;
        std::result::Result::Ok(token)
    }
    async fn username_available(&self, username: &str) -> bool {
        println!("username: {:?}", username);
        println!("{:?}", self.db.contains_key(username));
        self.db.contains_key(username)
    }
    async fn email_available(&self, email: &str) -> bool {
        let email: Result<EmailDb, sqlx::Error> =
            query_as(format!("SELECT * FROM valid.email WHERE email = {}", email).as_str())
                .fetch_one(&self.cockroach)
                .await;
        if let Err(err) = email {
            return true;
        }
        false
    }
    async fn hash_password(pwd: String) -> Result<(String, Uuid), Status> {
        let config = Config::default();
        let salt = Self::generate_salt();

        let hash = argon2::hash_encoded(pwd.as_bytes(), salt.as_bytes(), &config);
        if let Ok(hashed) = hash {
            Ok((hashed, salt))
        } else {
            Err(Status::invalid_argument("wrong password or username"))
        }
    }
    fn generate_salt() -> Uuid {
        Uuid::new_v4()
    }
    fn generate_id() -> Uuid {
        Uuid::new_v7(Timestamp::now(timestamp::context::NoContext))
    }

    async fn register_transaction(&self, req: Request<RegisterRequest>) -> AnyResult<User> {
        let created_at = LocalTime::now()
            .to_string()
            .parse::<DateTime<LocalTime>>()?;
        let creds = req.into_inner();
        let transaction = self.cockroach.begin().await?;

        let pwd = creds.password.clone();
        let (password, salt) = Self::hash_password(pwd).await?;
        let salt = sqlx::types::Uuid::parse_str(&salt.to_string())?;
        let username = creds.username.clone();
        let email = creds.email.clone();
        let lastname = creds.lastname.clone();
        let firstname = creds.firstname.clone();
        println!(" salt : {:?}", salt);

        let salt: Option<SaltDb> =
            query_as("with rows as (INSERT INTO valid.salt (salt) VALUES ($1) RETURNING id, salt) SELECT id, salt FROM rows")
                .bind(salt)
                .fetch_optional(&self.cockroach)
                .await?;
        println!("salt: {:?}", salt);
        // .map_err(|err| Error::new(err).context("salt issue"))?;
        let password: PasswordDb = query_as!(
            PasswordDb,
            "INSERT INTO valid.password (password) VALUES ($1) RETURNING *;",
            password
        )
        .fetch_one(&self.cockroach)
        .await?;
        let username: UsernameDb = query_as!(
            UsernameDb,
            " INSERT INTO valid.username (username) VALUES ($1) RETURNING *; ",
            username
        )
        .fetch_one(&self.cockroach)
        .await?;
        let email: EmailDb = query_as!(
            EmailDb,
            "INSERT INTO valid.email (email) VALUES ( $1 ) RETURNING *;",
            email
        )
        .fetch_one(&self.cockroach)
        .await
        .map_err(|err| Error::new(err).context("email issue"))?;
        let id = Self::generate_id();
        let user: UserDb = query_as!(UserDb,
                "INSERT INTO valid.users (id , email, username, password, salt, created_at, firstname, lastname) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING *;",
                id.clone(),
                email.id,
                username.id,
                password.id,
                salt.unwrap().id,
                created_at,
                firstname,
                lastname
        )
        .fetch_one(&self.cockroach)
        .await.map_err(|err| Error::new(err).context("User table issue"))?;
        transaction
            .commit()
            .await
            .map_err(|err| Error::new(err).context("transaction issue"))?;

        Ok(User {
            email: email.email,
            id,
            username: username.username,
        })
    }
}
#[derive(FromRow, Debug)]
struct RowId(i64);
#[derive(FromRow, Debug)]
struct SaltDb {
    id: i64,
    salt: Uuid,
}
#[derive(FromRow)]
struct PasswordDb {
    id: i64,
    password: String,
}
#[derive(FromRow)]
struct EmailDb {
    id: i64,
    email: String,
}
#[derive(FromRow, Debug)]
struct UsernameDb {
    id: i64,
    username: String,
}
#[derive(FromRow)]
struct UserDb {
    id: Uuid,
    email: i64,
    username: i64,
    password: i64,
    salt: i64,
    created_at: DateTime<LocalTime>,
    firstname: String,
    lastname: String,
}
