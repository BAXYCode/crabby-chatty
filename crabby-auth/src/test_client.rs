use crate::authenticate::auth::{authenticate_client::AuthenticateClient, RegisterRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = AuthenticateClient::connect("http://0.0.0.0:6869").await?;

    let register_test = tonic::Request::new(RegisterRequest {
        email: "lainebenjamin@gmail.com".to_string(),
        username: "BaxyDidIt".to_string(),
        password: "hahahtesoroito".to_string(),
        firstname: "Benjamin".to_string(),
        lastname: "Laine".to_string(),
    });
    let res = client.register(register_test).await?;
    println!("RESPONSE: {:?}", res);
    Ok(())
}
