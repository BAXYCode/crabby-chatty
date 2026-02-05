pub(crate) mod authenticate;
mod domain;
mod paseto;
mod users;
use std::sync::LazyLock;

use argon2::{Algorithm, Argon2, Params, Version};
use authenticate::auth::authenticate_server::AuthenticateServer;
use dotenvy::{dotenv, var};
use sqlx::PgPool;
use tonic::transport::Server;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
static ARGON2: LazyLock<Argon2> =
    LazyLock::new(|| Argon2::new(Algorithm::default(), Version::default(), Params::default()));
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // dotenv().expect("set .env file");
    let pgurl = var("DATABASE_URL").expect("Please provide postgres url as environment variable");

    let pg = PgPool::connect(pgurl.as_str()).await?;
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
    Server::builder()
        .add_service(AuthenticateServer::new(auth))
        .serve("0.0.0.0:6769".parse()?)
        .await?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use crate::authenticate::auth::{
        LoginRequest, RegisterRequest, authenticate_client::AuthenticateClient,
    };

    use eyre::{Ok, Result};
    use fake::{
        Fake,
        faker::{
            internet::en::{Password, SafeEmail, Username},
            name::en::{FirstName, LastName},
        },
    };
    use rand::Rng;
    use uuid::Uuid;
    #[derive(Clone)]
    struct RegisterTest {
        email: String,
        username: String,
        password: String,
    }
    struct LoginTest {
        username: String,
        password: String,
    }
    async fn get_client() -> Result<AuthenticateClient<tonic::transport::Channel>> {
        let client = AuthenticateClient::connect("http://0.0.0.0:6769").await?;
        Ok(client)
    }
    async fn generate_register_test() -> Result<RegisterTest> {
        Ok(RegisterTest {
            username: Username().fake(),
            email: SafeEmail().fake(),
            password: Password(8..32).fake(),
        })
    }
    impl LoginTest {
        fn login_to_request(self) -> LoginRequest {
            LoginRequest {
                username: self.username.clone(),
                password: self.password.clone(),
            }
        }
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

    #[tokio::test]
    async fn test_register() -> Result<()> {
        let mut client = get_client().await?;
        let register_test =
            tonic::Request::new(generate_register_test().await?.register_to_request());
        let res = client.register(register_test).await?;
        println!("RESPONSE: {:?}", res);
        Ok(())
    }
    #[tokio::test]
    async fn test_login() -> Result<()> {
        let mut client = get_client().await?;
        let register_baxy = RegisterTest {
            username: "BaxyDidIt".to_string(),
            password: "hahahtesoroito".to_string(),
            email: "whateveremail@gmail.com".to_string(),
        };
        let baxy_reg = client.register(register_baxy.register_to_request()).await?;
        let login_test = tonic::Request::new(LoginRequest {
            username: "BaxyDidIt".to_string(),
            password: "hahahtesoroito".to_string(),
        });
        let res = client.login(login_test).await?;
        println!("result: {:?}", res);
        Ok(())
    }
    #[tokio::test]
    async fn test_100_login_following_multi_register() -> Result<()> {
        let mut client = get_client().await?;
        let mut registers = Vec::new();
        let mut logins = Vec::new();
        for _ in 1..100 {
            let register = generate_register_test().await.unwrap();
            let login = register.register_to_login();
            registers.push(register);
            logins.push(login);
        }
        for i in registers {
            let register_test = tonic::Request::new(i.register_to_request());
            client.register(register_test).await?;
        }
        for i in logins {
            let login_test = tonic::Request::new(i.login_to_request());
            client.login(login_test).await?;
        }

        Ok(())
    }
}
