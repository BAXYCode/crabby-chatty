use dotenvy::{dotenv, var};
use pasetors::claims::ClaimsValidationRules;
pub struct ClaimsConfig {
    access: ClaimsValidationRules,
    refresh: ClaimsValidationRules,
}

impl ClaimsConfig {
    pub fn access(&self) -> &ClaimsValidationRules {
        &self.access
    }
    pub fn refresh(&self) -> &ClaimsValidationRules {
        &self.refresh
    }
    pub fn new() -> Self {
        let access_issuer = var("ACCESS_ISSUER").expect("ACCESS_ISSUER is needed");
        let refresh_issuer = var("REFRESH_ISSUER").expect("REFRESH_ISSUER is needed");
        let access_audience = var("ACCESS_AUDIENCE").expect("ACCESS_AUDIENCE is needed");
        let refresh_audience = var("REFRESH_AUDIENCE").expect("REFRESH_AUDIENCE is needed");

        let mut access = ClaimsValidationRules::new();
        let mut refresh = ClaimsValidationRules::new();
        access.validate_audience_with(&access_audience);
        access.validate_issuer_with(&access_issuer);
        refresh.validate_audience_with(&refresh_audience);
        refresh.validate_issuer_with(&refresh_issuer);

        Self { access, refresh }
    }
}
