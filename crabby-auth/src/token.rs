use anyhow::Result;
use chrono::{DateTime, Duration, TimeDelta, Utc};
use pasetors::{
    claims::{Claims, ClaimsValidationRules},
    footer::Footer,
    keys::{AsymmetricKeyPair, AsymmetricPublicKey, AsymmetricSecretKey, SymmetricKey},
    local,
    paserk::Id,
    public,
    token::{TrustedToken, UntrustedToken},
    version4::V4,
    Local, Public,
};
use tonic::Status;
use uuid::Uuid;
pub trait Verify {
    type Token;
    fn verify(&mut self, token: &Self::Token) -> Result<TrustedToken, Status>;
    fn verify_version(&self, token: &Self::Token) -> Result<TokenVersion, Status>;
    fn verify_type(&self, token: &Self::Token) -> Result<TokenType, Status>;
}
//Blanket implementation of the Verify trait for when the token type is a string
pub trait VerifyString: Verify<Token = String> {
    fn verify_version(&self, token: &Self::Token) -> Result<TokenVersion, Status> {
        let split_token = token.split(".").collect::<Vec<&str>>();
        match split_token[0] {
            "v4" => Ok(TokenVersion::V4),
            "v3" => Ok(TokenVersion::V3),
            "v2" => Ok(TokenVersion::V2),
            "v1" => Ok(TokenVersion::V1),
            _ => Err(Status::invalid_argument("Invalid token version")),
        }
    }
    fn verify_type(&self, token: &Self::Token) -> Result<TokenType, Status> {
        let split_token = token.split(".").collect::<Vec<&str>>();
        match split_token[1] {
            "local" => Ok(TokenType::Local),
            "public" => Ok(TokenType::Public),
            _ => Err(Status::invalid_argument("Invalid token type")),
        }
    }
}
pub enum TokenType {
    Local,
    Public,
}
impl From<Local> for TokenType {
    fn from(_value: Local) -> Self {
        Self::Local
    }
}
impl From<Public> for TokenType {
    fn from(_value: Public) -> Self {
        Self::Public
    }
}
//All token versions available if ever they are needed in the futur
pub enum TokenVersion {
    V4,
    V3,
    V2,
    V1,
}
impl From<V4> for TokenVersion {
    fn from(_value: V4) -> Self {
        Self::V4
    }
}
//Newtype pattern over pasetors keys
pub enum VerificationKey<T> {
    Symmetric(SymmetricKey<T>),
    Asymmetric(AsymmetricPublicKey<T>),
}
impl Clone for VerificationKey<V4> {
    fn clone(&self) -> Self {
        match self {
            Self::Symmetric(arg0) => Self::Symmetric(arg0.clone()),
            Self::Asymmetric(arg0) => Self::Asymmetric(arg0.clone()),
        }
    }
}
pub struct TokenVerificationConfig<T> {
    key: VerificationKey<T>,
    validation_rules: Option<ClaimsValidationRules>,
}
impl TokenVerificationConfig<V4> {
    fn symmetric(key: SymmetricKey<V4>) -> Self {
        Self {
            key: VerificationKey::Symmetric(key),
            validation_rules: None,
        }
    }
    fn asymmetric(key: AsymmetricPublicKey<V4>) -> Self {
        Self {
            key: VerificationKey::Asymmetric(key),
            validation_rules: None,
        }
    }
    fn with_validation_rules(&mut self, rules: ClaimsValidationRules) {
        self.validation_rules = Some(rules)
    }
    fn with_asymmetric_key(&mut self, key: AsymmetricPublicKey<V4>) {
        self.key = VerificationKey::Asymmetric(key)
    }
    fn with_symmetric_key(&mut self, key: SymmetricKey<V4>) {
        self.key = VerificationKey::Symmetric(key)
    }
}
pub struct AuthTokenVerifier<T> {
    config: TokenVerificationConfig<T>,
    expiry: bool,
    leeway: Option<Duration>,
}
impl VerifyString for AuthTokenVerifier<pasetors::version4::V4> {}
impl Verify for AuthTokenVerifier<pasetors::version4::V4> {
    type Token = String;

    fn verify(&mut self, token: &Self::Token) -> Result<TrustedToken, Status> {
        let token_type = Verify::verify_type(self, &token)?;
        match token_type {
            TokenType::Local => self.verify_public(&token),
            TokenType::Public => self.verify_local(&token),
        }
    }

    fn verify_type(&self, token: &Self::Token) -> Result<TokenType, Status> {
        VerifyString::verify_type(self, token)
    }

    fn verify_version(&self, token: &Self::Token) -> Result<TokenVersion, Status> {
        VerifyString::verify_version(self, token)
    }
}
//INFO: Maybe there is a more ergonomic way to do this
impl AuthTokenVerifier<V4> {
    //Verifies validity of public token without checking custom claims
    fn verify_public(&mut self, token: &str) -> Result<TrustedToken, Status> {
        let untrusted: UntrustedToken<Public, V4> = UntrustedToken::try_from(token)
            .map_err(|_err| Status::invalid_argument("Invalid public token"))?;
        //Extract footer from untrusted token
        let mut footer = Footer::new();
        footer.parse_bytes(untrusted.untrusted_footer());
        let config = self.config.validation_rules.take().unwrap();
        match self.config.key.clone() {
            VerificationKey::Asymmetric(key) => {
                public::verify(&key, &untrusted, &config, Some(&footer), None)
                    .map_err(|_err| Status::invalid_argument("invalid public token"))
            }
            VerificationKey::Symmetric(_key) => {
                Err(Status::internal("internal error when verifying token"))
            }
        }
    }
    //Verifies validity of local token without checking custom claims
    fn verify_local(&mut self, token: &str) -> Result<TrustedToken, Status> {
        let untrusted: UntrustedToken<Local, V4> = UntrustedToken::try_from(token)
            .map_err(|_err| Status::invalid_argument("Invalid local token"))?;
        //Extract footer from untrusted token
        let mut footer = Footer::new();
        footer.parse_bytes(untrusted.untrusted_footer());
        let config = self.config.validation_rules.take().unwrap();
        match self.config.key.clone() {
            VerificationKey::Asymmetric(_key) => {
                Err(Status::internal("internal error when verifying token"))
            }
            VerificationKey::Symmetric(key) => {
                local::decrypt(&key, &untrusted, &config, Some(&footer), None)
                    .map_err(|_err| Status::invalid_argument("invalid local token"))
            }
        }
    }
    fn verify_token(&mut self, token: &str) {}
}
impl<T> AuthTokenVerifier<T> {
    pub fn verification_config(config: TokenVerificationConfig<T>) -> Self {
        Self {
            config,
            expiry: false,
            leeway: None,
        }
    }
    pub fn with_leeway(mut self, duration: Duration) -> Self {
        self.leeway = Some(duration);
        self
    }
    pub fn expiry(mut self, expiry: bool) -> Self {
        self.expiry = expiry;
        self
    }
}
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
    pub(crate) refresh: RefreshTokenWithMetadata,
    pub(crate) bearer: String,
}
impl UserTokens {
    pub(crate) fn new(bearer: String, refresh: RefreshTokenWithMetadata) -> Self {
        Self { refresh, bearer }
    }
}
//FIX: better error handling is required
pub(super) fn bearer(
    username: &str,
    id: &str,
    admin: bool,
    key: &AsymmetricKeyPair<V4>,
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
    //add public key id for database lookup for when key rotations are implemented
    let key_id = Id::from(&key.public);
    let mut footer = Footer::new();
    footer.key_id(&key_id);
    let token = public::sign(&key.secret, &claims, Some(&footer), None).expect("encode paseto");

    Ok(token)
}
pub(crate) struct RefreshTokenWithMetadata {
    token: String,
    pub iat: DateTime<Utc>,
    pub nbf: DateTime<Utc>,
    pub exp: DateTime<Utc>,
    pub token_id: Uuid,
}
impl RefreshTokenWithMetadata {
    pub fn token(&self) -> String {
        self.token.clone()
    }
}
pub(crate) fn refresh(key: &SymmetricKey<V4>) -> Result<RefreshTokenWithMetadata, Status> {
    //Set refresh token to expire in 14 days, arbitrary value, can set in better way
    let delta = TimeDelta::days(14);
    let now = Utc::now();
    //INFO: Arbitrarily set not before delta time, could be set through environment variables or config
    let not_before = now + TimeDelta::seconds(30);
    let expiry = now + delta;
    //unwrap is safe here as it's unlikely that this program will be running when time reaches
    //out of range
    let mut claims = Claims::new().unwrap();
    claims.expiration(expiry.to_rfc3339().as_str()).unwrap();
    claims.issued_at(now.to_rfc3339().as_str());
    claims.not_before(not_before.to_rfc3339().as_str());
    let _ = claims.issuer("Baxy");
    //FIX: find a better way to generate IDs
    let token_id = Uuid::new_v4();
    let mut footer = Footer::new();
    //INFO: value is not any of the reserved keywords for paseto, safe to unwrap
    footer
        .add_additional("refresh_id", token_id.to_string().as_str())
        .unwrap();
    let token = local::encrypt(key, &claims, Some(&footer), None).expect("refresh paseto");
    let token_metadata = RefreshTokenWithMetadata {
        token,
        iat: now,
        nbf: not_before,
        exp: expiry,
        token_id,
    };
    Ok(token_metadata)
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
