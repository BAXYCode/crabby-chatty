use pasetors::{Local, token::UntrustedToken, version4::V4};
use tonic::{Request, Status};

pub fn intercept(mut req: Request<()>) -> Result<Request<()>, Status> {
    let token = req
        .metadata()
        .get("authorization")
        .or_else(|| req.metadata().get("Authorization"));

    if let Some(token) = token {
        let parsed = parse_auth_token(
            token
                .to_str()
                .map_err(|_| Status::internal("internal issue"))?,
        )?;
        req.extensions_mut().insert(parsed);
    }
    Ok(req)
}
fn parse_auth_token(header_contents: &str) -> Result<TokenExtension, Status> {
    let stripped: &str = header_contents
        .strip_prefix("Bearer ")
        .ok_or_else(|| Status::unauthenticated("invalid token"))?;
    Ok(TokenExtension {
        token_string: stripped.to_string(),
    })
}
#[derive(Clone)]
pub struct TokenExtension {
    pub token_string: String,
}
impl TokenExtension {
    pub fn into_inner(self) -> String {
        self.token_string
    }
}
