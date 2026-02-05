use ::core::time::Duration;
use chrono::{DateTime, TimeDelta, Utc};
use eyre::Result;
use pasetors::{
    claims::Claims,
    footer::{self, Footer},
    keys::{AsymmetricKeyPair, SymmetricKey},
    local,
    paserk::Id,
    public,
    version4::V4,
};
use std::str::FromStr;

use tonic::Status;
use uuid::Uuid;

use crate::domain::models::RefreshTokenWithMetadata;
pub(crate) struct UserTokens {
    pub(crate) refresh: RefreshTokenWithMetadata,
    pub(crate) bearer: String,
}
impl UserTokens {
    pub(crate) fn new(bearer: String, refresh: RefreshTokenWithMetadata) -> Self {
        Self { refresh, bearer }
    }
}
//FIX: better error handling is required
pub(crate) fn bearer(
    username: &str,
    id: &str,
    key: &AsymmetricKeyPair<V4>,
) -> Result<String, Status> {
    //Set bearer token to expire in 15 minutes
    let delta = Duration::from_mins(15);
    let mut claims = Claims::new().unwrap();

    let _ = claims.set_expires_in(&delta);
    let _ = claims.issuer("crabby-auth");
    let _ = claims.audience("crabby-gateway");
    let _ = claims.subject(id);
    let _ = claims.add_additional("username", username);
    // let _ = claims.add_additional("admin", admin);
    //TODO:  Implement proper ID generation to identify public keys for verification of signature
    let key_id = Id::from(&key.public);
    let mut footer = Footer::new();
    footer.key_id(&key_id);
    let token = public::sign(&key.secret, &claims, Some(&footer), None).expect("encode paseto");

    Ok(token)
}
pub(crate) fn refresh(
    key: &SymmetricKey<V4>,
    id: &str,
) -> Result<RefreshTokenWithMetadata, Status> {
    //Set refresh token to expire in 14 days, arbitrary value, can set in better way
    let delta = Duration::from_hours(336);
    let now = Utc::now();
    let jti = Uuid::new_v4();
    //INFO: Arbitrarily set not before delta time, could be set through environment variables or config
    let expiry = now + delta;
    //unwrap is safe here as it's unlikely that this program will be running when time reaches
    //out of range
    let mut claims = Claims::new().unwrap();
    claims.set_expires_in(&delta);
    let _ = claims.issuer("crabby-auth");
    let _ = claims.audience("crabby-auth");
    let _ = claims.subject(id);
    let _ = claims.add_additional("jti", jti.to_string());

    let mut footer = footer::Footer::new();
    footer.key_id(&Id::from(key));
    //INFO: value is not any of the reserved keywords for paseto, safe to unwrap

    let token = local::encrypt(key, &claims, Some(&footer), None).expect("refresh paseto");
    let token_metadata = RefreshTokenWithMetadata {
        token,
        jti,
        user_id: Uuid::from_str(id).unwrap(),
        issued_at: now,
        exp: expiry,
    };
    Ok(token_metadata)
}
