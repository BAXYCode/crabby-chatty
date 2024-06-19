mod authenticate;
mod model;
use std::env;

use authenticate::auth::authenticate_server::AuthenticateServer;
use sqlx::PgPool;
use tonic::transport::Server;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pgurl = env::var("PG_URL").expect("Please provide postgres url as environment variable");
    let url = "postgresql://root@localhost:26257/defaultdb?sslmode=disable";
    let pg = PgPool::connect(pgurl.as_str()).await?;
    sqlx::migrate!("db/migrations")
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
        .serve("0.0.0.0:6869".parse()?)
        .await?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use crate::authenticate::auth::{
        authenticate_client::AuthenticateClient, LoginRequest, RegisterRequest,
    };

    use anyhow::{Ok, Result};
    use faker_rand::en_us::{
        internet::{Email, Username},
        names::{FirstName, LastName},
    };
    use rand::Rng;
    use uuid::Uuid;
    #[derive(Clone)]
    struct RegisterTest {
        email: String,
        username: String,
        password: String,
        firstname: String,
        lastname: String,
    }
    struct LoginTest {
        username: String,
        password: String,
    }
    async fn get_client() -> anyhow::Result<AuthenticateClient<tonic::transport::Channel>> {
        let client = AuthenticateClient::connect("http://0.0.0.0:6869").await?;
        Ok(client)
    }
    async fn generate_register_test() -> anyhow::Result<RegisterTest> {
        let mut random = rand::thread_rng();
        let firstname = random.gen::<FirstName>().to_string();
        let lastname = random.gen::<LastName>().to_string();
        let email = random.gen::<Email>().to_string();
        let password = Uuid::new_v4().to_string();
        let username = random.gen::<Username>().to_string();
        Ok(RegisterTest {
            username,
            firstname,
            lastname,
            email,
            password,
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
                firstname: self.firstname.clone(),
                lastname: self.lastname.clone(),
            }
        }
    }

    #[tokio::test]
    async fn test_register() -> anyhow::Result<()> {
        let mut client = get_client().await?;

        let register_test =
            tonic::Request::new(generate_register_test().await?.register_to_request());
        let res = client.register(register_test).await?;
        println!("RESPONSE: {:?}", res);
        Ok(())
    }
    #[tokio::test]
    async fn test_login() -> anyhow::Result<()> {
        let mut client = get_client().await?;
        let login_test = tonic::Request::new(LoginRequest {
            username: "BaxyDidIt".to_string(),
            password: "hahahtesoroito".to_string(),
        });
        let res = client.login(login_test).await?;
        println!("result: {:?}", res);
        Ok(())
    }
    //INFO: this test can fail because the generator I use for some of the variables isn't general
    //enough and it can create duplicate entries.
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
