use serde::Deserialize;
use validator::Validate;

#[derive(Clone, Validate, Deserialize)]
#[serde(transparent)]
pub struct EmailAddress {
    #[validate(email)]
    email: String,
}
//Newtype for username validation
#[derive(Clone, Validate, Deserialize)]
#[serde(transparent)]
pub struct Username {
    #[validate(length(min = 5, max = 16))]
    username: String,
}
// DTO for register requests
pub struct RegisterRequestData {}
