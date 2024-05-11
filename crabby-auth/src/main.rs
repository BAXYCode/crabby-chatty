mod authenticate;
use authenticate::auth::authenticate_server::AuthenticateServer;
use sqlx::PgPool;
use tonic::transport::Server;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = "postgresql://root@roach2:26257/defaultdb?sslmode=disable";
    let pg = PgPool::connect(url).await?;
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
    use crate::authenticate::auth::{authenticate_client::AuthenticateClient, RegisterRequest};
    #[tokio::test]
    async fn test_register() {
        let mut client = AuthenticateClient::connect("http://0.0.0.0:6869")
            .await
            .unwrap();

        let register_test = tonic::Request::new(RegisterRequest {
            email: "lainebenjamin@gmail.com".to_string(),
            username: "BaxyDidIt".to_string(),
            password: "hahahtesoroito".to_string(),
        });
        let res = client.register(register_test).await;
        if let Ok(res) = res {
            println!("RESPONSE: {:?}", res);
        } else {
            println!("ERR: {:?}", res.expect_err("wtf"))
        }
    }
}
