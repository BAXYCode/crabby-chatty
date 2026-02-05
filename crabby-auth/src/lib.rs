pub(crate) mod authenticate;
mod domain;
mod paseto;
mod users;
use argon2::{Algorithm, Argon2, Params, Version};
use std::sync::LazyLock;
static ARGON2: LazyLock<Argon2> =
    LazyLock::new(|| Argon2::new(Algorithm::default(), Version::default(), Params::default()));
