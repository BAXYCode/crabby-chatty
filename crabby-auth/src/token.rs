use anyhow::Result;
use chrono::{TimeDelta, Utc};
use pasetors::{
    claims::{Claims, ClaimsValidationRules},
    keys::SymmetricKey,
    local,
    token::{TrustedToken, UntrustedToken},
    version4::V4,
    Local,
};
use tonic::Status;
pub(crate) struct UnverifiedRefreshToken {
    token: String,
}
impl UnverifiedRefreshToken {
    pub fn new(token: String) -> Self {
        Self { token }
    }
}
pub(crate) struct UnverifiedWithKey<T> {
    token: T,
    key: SymmetricKey<V4>,
}
impl<T> UnverifiedWithKey<T> {
    pub fn new(token: T, key: SymmetricKey<V4>) -> Self {
        Self { token, key }
    }
}
pub(crate) struct UserTokens {
    pub(crate) refresh: String,
    pub(crate) bearer: String,
}
impl UserTokens {
    pub(crate) fn new(bearer: String, refresh: String) -> Self {
        Self { refresh, bearer }
    }
}
//FIX: better error handling is required
pub(super) fn bearer(
    username: &str,
    id: &str,
    admin: bool,
    key: &SymmetricKey<V4>,
) -> Result<String, Status> {
    //Set bearer token to expire in 15 minutes
    let delta = TimeDelta::minutes(15);
    //INFO:unwrap is safe here as it's unlikely that this program will be running when time reaches
    //out of range
    let expiry = Utc::now() + delta;
    let mut claims = Claims::new().unwrap();
    claims
        .expiration(expiry.to_rfc3339().as_str())
        .expect("parsing issue expiry");
    let _ = claims.issuer("Baxy");
    let _ = claims.add_additional("username", username);
    let _ = claims.add_additional("id", id);
    let _ = claims.add_additional("admin", admin);

    let token = local::encrypt(key, &claims, None, None).expect("encode paseto");

    Ok(token)
}

pub(crate) fn refresh(key: &SymmetricKey<V4>, version: i64) -> Result<String, Status> {
    //Set refresh token to expire in 14 days
    let delta = TimeDelta::days(14);
    //unwrap is safe here as it's unlikely that this program will be running when time reaches
    //out of range
    let expiry = Utc::now() + delta;
    let mut claims = Claims::new().unwrap();
    claims.expiration(expiry.to_rfc3339().as_str()).unwrap();
    let _ = claims.add_additional("version", version);
    let _ = claims.issuer("Baxy");

    let token = local::encrypt(key, &claims, None, None).expect("refresh paseto");
    Ok(token)
}
//Check if token is expired
pub(crate) fn expired<T: Token>(unverified: &UnverifiedWithKey<T>) -> Result<bool, Status> {
    let maybe_valid = validate_token(unverified);
    if let Err(err) = maybe_valid {
        match err {
            pasetors::errors::Error::ClaimValidation(
                pasetors::errors::ClaimValidationError::Exp,
            ) => return Ok(true),
            _ => return Err(Status::deadline_exceeded("Internal error")),
        }
    }

    Ok(false)
}
pub(crate) fn validate_refresh<T: Token>(
    unverified: &UnverifiedWithKey<T>,
    version: i64,
) -> Result<(), pasetors::errors::Error> {
    let trusted = validate_token(unverified)?;
    let claims = trusted.payload_claims().unwrap();
    //FIX: clean this up
    if let Some(claimed_version) = claims.get_claim("version") {
        if let Some(claimed_version) = claimed_version.as_str() {
            if claimed_version
                .parse::<i64>()
                .is_ok_and(|version_in_claim| version_in_claim == version)
            {
                return Ok(());
            }
        }
    }
    Err(pasetors::errors::Error::TokenValidation)
}

fn validate_token<T: Token>(
    unverified: &UnverifiedWithKey<T>,
) -> Result<TrustedToken, pasetors::errors::Error> {
    let mut validation_rules = ClaimsValidationRules::new();
    validation_rules.disable_valid_at();
    validation_rules.validate_issuer_with("Baxy");
    let untrusted = UntrustedToken::<Local, V4>::try_from(&unverified.token()).unwrap();
    Ok(local::decrypt(
        &unverified.key,
        &untrusted,
        &validation_rules,
        None,
        None,
    )?)
}

pub(crate) trait Token {
    fn token(&self) -> String;
}
impl Token for UnverifiedRefreshToken {
    fn token(&self) -> String {
        self.token.clone()
    }
}
impl<T: Token> Token for UnverifiedWithKey<T> {
    fn token(&self) -> String {
        self.token.token()
    }
}
