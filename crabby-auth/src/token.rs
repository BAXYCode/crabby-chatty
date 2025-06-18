use anyhow::Result;
use chrono::{DateTime, Duration, TimeDelta, Utc};
use pasetors::{
    claims::{self, Claims, ClaimsValidationRules},
    keys::SymmetricKey,
    local,
    token::{TrustedToken, UntrustedToken},
    version4::V4,
    Local,
};
use tonic::Status;
pub(crate) struct RefreshTokenWithVersion {
    token: String,
    version: usize,
}
pub(crate) struct UnverifiedWithKey<T> {
    token: T,
    key: SymmetricKey<V4>,
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
    claims.issuer("Baxy");
    claims.add_additional("username", username);
    claims.add_additional("id", id);
    claims.add_additional("admin", admin);

    let token = local::encrypt(key, &claims, None, None).expect("encode paseto");

    Ok(token)
}

pub(crate) fn refresh(key: &SymmetricKey<V4>, version: &str) -> Result<String, Status> {
    //Set refresh token to expire in 14 days
    let delta = TimeDelta::days(14);
    //unwrap is safe here as it's unlikely that this program will be running when time reaches
    //out of range
    let expiry = Utc::now() + delta;
    let mut claims = Claims::new().unwrap();
    claims.expiration(expiry.to_rfc3339().as_str()).unwrap();
    claims.add_additional("version", version);
    claims.issuer("Baxy");

    let token = local::encrypt(key, &claims, None, None).expect("refresh paseto");
    Ok(token)
}
//Check if token is expired
pub(crate) fn expired(token: String, key: &SymmetricKey<V4>) -> Result<bool, Status> {
    let maybe_valid = validate_token(token, key);
    if let Err(err) = maybe_valid {
        match err {
            pasetors::errors::Error::ClaimValidation(
                pasetors::errors::ClaimValidationError::Exp,
            ) => return Ok(true),
            _ => return Err(Status::unavailable("Internal error")),
        }
    }

    Ok(false)
}
pub(crate) fn validate_refresh(
    token: RefreshTokenWithVersion,
    key: &SymmetricKey<V4>,
) -> Result<(), Status> {
    let trusted = validate_token(token.token, key).map_err(|err| Status::aborted("failed"))?;
    let claims = trusted.payload_claims().unwrap();
    let version = claims.get_claim("version").unwrap();
    Ok(())
}

fn validate_token(
    token: String,
    key: &SymmetricKey<V4>,
) -> Result<TrustedToken, pasetors::errors::Error> {
    let mut validation_rules = ClaimsValidationRules::new();
    validation_rules.disable_valid_at();
    validation_rules.validate_issuer_with("Baxy");
    let untrusted = UntrustedToken::<Local, V4>::try_from(&token).unwrap();
    Ok(local::decrypt(
        key,
        &untrusted,
        &validation_rules,
        None,
        None,
    )?)
}

trait Token {
    fn token() -> String;
}
