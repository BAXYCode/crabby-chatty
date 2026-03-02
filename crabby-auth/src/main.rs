pub mod authenticate;
pub mod domain;
pub mod intercept;
pub mod paseto;
pub mod users;
use std::{sync::LazyLock, time::Duration};

use argon2::{Algorithm, Argon2, Params, Version};
use authenticate::auth::authenticate_server::AuthenticateServer;
use dotenvy::{dotenv, var};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use tonic::transport::Server;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

static ARGON2: LazyLock<Argon2> =
    LazyLock::new(|| Argon2::new(Algorithm::default(), Version::default(), Params::default()));
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // dotenv().expect("set .env file");
    let pgurl = var("DATABASE_URL").expect("Please provide postgres url as environment variable");

    let pg = PgPoolOptions::new()
        .max_connections(128)
        .min_connections(16)
        .acquire_timeout(Duration::from_secs(2))
        .connect(&pgurl)
        .await?;
    sqlx::migrate!("./migrations")
        .set_locking(false)
        .run(&pg)
        .await?;
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::filter::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                // axum logs rejections from built-in extractors with the `axum::rejection`
                // target, at `TRACE` level. `axum::rejection=trace` enables showing those events
                "example_tracing_aka_logging=debug,tower_http=debug,axum::rejection=trace".into()
            }),
        )
        .with(tracing_subscriber::fmt::layer().pretty())
        .init();

    let auth = authenticate::Authenticator::new(pg);
    let with_interceptor = AuthenticateServer::with_interceptor(auth, intercept::intercept);
    Server::builder()
        .add_service(with_interceptor)
        .serve("0.0.0.0:6769".parse()?)
        .await?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use once_cell::sync::Lazy;

    use crate::authenticate::auth::{
        LoginRequest, RefreshRequest, RegisterRequest, authenticate_client::AuthenticateClient,
    };

    use eyre::Result;
    use fake::{Fake, faker::internet::en::Password};

    // Tune this:
    // - Must be >= the number of users you'll need across all tests
    // - If you later run tests in parallel with `cargo test`, bump it
    const USER_POOL_SIZE: usize = 10_000;

    #[derive(Clone, Debug)]
    struct RegisterTest {
        email: String,
        username: String,
        password: String,
    }

    #[derive(Clone, Debug)]
    struct LoginTest {
        username: String,
        password: String,
    }

    impl RegisterTest {
        fn register_to_login(&self) -> LoginTest {
            LoginTest {
                username: self.username.clone(),
                password: self.password.clone(),
            }
        }

        fn register_to_request(&self) -> RegisterRequest {
            RegisterRequest {
                email: self.email.clone(),
                username: self.username.clone(),
                password: self.password.clone(),
            }
        }
    }

    impl LoginTest {
        fn login_to_request(&self) -> LoginRequest {
            LoginRequest {
                username: self.username.clone(),
                password: self.password.clone(),
            }
        }
    }

    // 1) Generate data ONCE, synchronously, before tests hammer your async runtime.
    static USERS: Lazy<Vec<RegisterTest>> = Lazy::new(|| {
        let mut out = Vec::with_capacity(USER_POOL_SIZE);

        for i in 0..USER_POOL_SIZE {
            // Deterministically unique + valid-ish:
            let username = format!("user{i}");
            let email = format!("user{i}@example.test");

            // Option A: fixed password (fastest + reproducible)
            // let password = "password123!".to_string();

            // Option B: random password (still cheap)
            let password: String = Password(12..24).fake();

            out.push(RegisterTest {
                email,
                username,
                password,
            });
        }

        out
    });

    // 2) Index into the pre-generated pool. No locks; just atomic fetch_add.
    static USER_IDX: AtomicUsize = AtomicUsize::new(0);

    fn next_register_test() -> RegisterTest {
        let idx = USER_IDX.fetch_add(1, Ordering::Relaxed);
        USERS
            .get(idx % USERS.len())
            .expect("USER_POOL_SIZE must be > 0")
            .clone()
    }

    async fn get_client() -> Result<AuthenticateClient<tonic::transport::Channel>> {
        Ok(AuthenticateClient::connect("http://0.0.0.0:6769").await?)
    }

    fn refresh_request_with_auth(refresh_token: &str) -> tonic::Request<RefreshRequest> {
        let mut req = tonic::Request::new(RefreshRequest {
            refresh: String::new(),
            user_id: String::new(),
        });

        let header_val = format!("Bearer {refresh_token}");
        req.metadata_mut().insert(
            "authorization",
            header_val.parse().expect("valid metadata value"),
        );
        req
    }

    #[tokio::test]
    async fn test_register() -> Result<()> {
        let mut client = get_client().await?;
        let reg = next_register_test();
        let res = client
            .register(tonic::Request::new(reg.register_to_request()))
            .await?;
        println!("RESPONSE: {:?}", res);
        Ok(())
    }

    #[tokio::test]
    async fn test_login() -> Result<()> {
        let mut client = get_client().await?;

        // Ensure user exists
        let reg = next_register_test();
        client
            .register(tonic::Request::new(reg.register_to_request()))
            .await?;

        let login = reg.register_to_login();
        let res = client
            .login(tonic::Request::new(login.login_to_request()))
            .await?;
        println!("result: {:?}", res);
        Ok(())
    }

    #[tokio::test]
    async fn test_refresh_uses_authorization_header() -> Result<()> {
        let mut client = get_client().await?;

        let reg = next_register_test();
        let reg_res = client
            .register(tonic::Request::new(reg.register_to_request()))
            .await?
            .into_inner();

        let refresh_token = reg_res
            .response
            .as_ref()
            .expect("register success")
            .refresh
            .clone();

        let res = client
            .refresh(refresh_request_with_auth(&refresh_token))
            .await;
        println!("{:?}", res);
        assert!(res.is_ok(), "refresh should succeed: {res:?}");

        Ok(())
    }
}
