use eyre::Result;
use pasetors::token::TrustedToken;

/*This trait will eventually be used by any service that will require token based authentication
I am adding the trait bound for KeyRetrieval because in order to verify the validity of the token,
a key is always required regardless of it being a public key or a local decryption key*/
pub trait VerifyToken<T, V, K>
where
    Self::Storage: KeyRetrieval<K>,
{
    type Storage;
    async fn verify(&mut self, token: String) -> Result<TrustedToken>;
}
pub trait KeyRetrieval<K> {
    async fn get_key(&self, kid: &str) -> Result<K>;
}
// pub struct<R> TokenVerifier<K>
// where
//     R: KeyStorage<K>,
// {
//     validation_rules: ClaimsValidationRules,
//     retreiver: R,
//
//     // leeway: Option<Duration>,
//     phantom: PhantomData<K>,
// }

// impl<PostgresKeyStorage> VerifyToken<Local, V4>
//     for TokenVerifier<PostgresKeyStorage, SymmetricKey<V4>>
// {
//     async fn verify(
//         &mut self,
//         token: String,
//     ) -> std::result::Result<TrustedToken, pasetors::errors::Error> {
//         let untrusted: UntrustedToken<Local, V4> = UntrustedToken::try_from(&token)?;
//         //Extract footer from untrusted token
//         let mut footer = Footer::new();
//         footer.parse_bytes(untrusted.untrusted_footer());
//         //extract k_id from footer
//         let kid = footer
//             .get_claim("kid")
//             .and_then(|a| a.as_str())
//             .ok_or(pasetors::errors::Error::TokenValidation)?;
//         //fetch k_id from storage if exists
//         let key = self
//             .retreiver
//             .get_key(kid)
//             .await
//             .map_err(|e| pasetors::errors::Error::TokenValidation)?;
//         let result = local::decrypt(
//             &key,
//             &untrusted,
//             &self.validation_rules,
//             Some(&footer),
//             None,
//         );
//         result
//     }
// }
