use std::sync::Arc;

use crabby_core::tokens::KeyRetrieval;
use eyre::Result;
use hmac::{Hmac, Mac};
use pasetors::{
    keys::{AsymmetricPublicKey, SymmetricKey},
    paserk::{FormatAsPaserk, Id},
    version4::V4,
};
use sha2::Sha256;
use sqlx::{PgPool, prelude::FromRow, query, query_as};

use crate::domain::models::RefreshTokenWithMetadata;
pub struct PostgresKeyRepo {
    pub conn: Arc<PgPool>,
}

#[derive(FromRow)]
struct KeyBytesPublic {
    public_paserk: Vec<u8>,
}
impl KeyRetrieval<AsymmetricPublicKey<V4>> for PostgresKeyRepo {
    async fn get_key(&self, kid: &str) -> Result<AsymmetricPublicKey<V4>> {
        let mut tx = self.conn.begin().await?;
        let bytes = query_as!(
            KeyBytesPublic,
            "SELECT public_paserk FROM  validation.paseto_public_key  WHERE kid= ( $1 )",
            kid
        )
        .fetch_one(&mut *tx)
        .await?;
        let key = AsymmetricPublicKey::from(bytes.public_paserk.as_slice());
        tx.commit().await;
        Ok(key?)
    }
}
#[derive(FromRow)]
struct KeyBytes {
    local_wrap_paserk: Vec<u8>,
}
impl KeyRetrieval<SymmetricKey<V4>> for PostgresKeyRepo {
    async fn get_key(&self, kid: &str) -> Result<SymmetricKey<V4>> {
        let mut tx = self.conn.begin().await?;
        let bytes = query_as!(
            KeyBytes,
            "SELECT local_wrap_paserk FROM  validation.paseto_local_wrap_key WHERE kid= ( $1 )",
            kid
        )
        .fetch_one(&mut *tx)
        .await?;
        let key = SymmetricKey::from(&bytes.local_wrap_paserk.as_slice());
        tx.commit().await;
        Ok(key?)
    }
}

pub trait PasetoKeyRepo {
    async fn store_public_key(&self, key: AsymmetricPublicKey<V4>) -> Result<()>;
    async fn fetch_public_key(&self, kid: String) -> Result<AsymmetricPublicKey<V4>>;
    async fn store_local_key(&self, key: SymmetricKey<V4>) -> Result<()>;
    async fn fetch_local_key(&self, kid: String) -> Result<SymmetricKey<V4>>;
    async fn store_refresh_info(&self, token: &RefreshTokenWithMetadata) -> Result<()>;
}

impl PasetoKeyRepo for PostgresKeyRepo {
    async fn store_public_key(&self, key: AsymmetricPublicKey<V4>) -> Result<()> {
        let id = Id::from(&key);

        let mut string_id = String::new();
        id.fmt(&mut string_id);
        let mut tx = self.conn.begin().await?;
        let key_bytes = key.as_bytes();
        let _ = query!(
            "INSERT INTO validation.paseto_public_key (kid, public_paserk) VALUES ($1,$2) RETURNING kid ",
            string_id,
            key_bytes
        )
        .fetch_one(&mut *tx).await?;
        tx.commit().await;
        Ok(())
    }

    async fn fetch_public_key(&self, kid: String) -> Result<AsymmetricPublicKey<V4>> {
        <Self as KeyRetrieval<AsymmetricPublicKey<V4>>>::get_key(self, kid.as_str()).await
    }
    //Lots of cleaning up TODO:
    async fn store_local_key(&self, key: SymmetricKey<V4>) -> Result<()> {
        let mut tx = self.conn.begin().await?;
        let id = Id::from(&key);
        let mut string_id = String::new();
        id.fmt(&mut string_id);
        let key_bytes = key.as_bytes();
        query!(
            "INSERT INTO validation.paseto_local_wrap_key (kid, local_wrap_paserk) VALUES ($1,$2) RETURNING kid ",
            string_id,
            key_bytes
        )
        .fetch_one(&*self.conn).await?;
        tx.commit().await;
        Ok(())
    }

    async fn fetch_local_key(&self, kid: String) -> Result<SymmetricKey<V4>> {
        <Self as KeyRetrieval<SymmetricKey<V4>>>::get_key(self, kid.as_str()).await
    }

    async fn store_refresh_info(&self, token: &RefreshTokenWithMetadata) -> Result<()> {
        let mut tx = self.conn.begin().await?;
        type HmacSha256 = Hmac<Sha256>;

        let hash = HmacSha256::new_from_slice(token.token().as_bytes())
            .expect("hasher")
            .finalize();
        let bytes = hash.into_bytes();
        let _ = query!(
            "INSERT INTO validation.refresh_token(user_id, token_jti, token_hash, issued_at, expires_at) VALUES ($1,$2, $3, $4, $5) ",
            token.user_id,&token.jti, bytes.as_slice(), token.issued_at, token.exp
        )
        .execute(&mut *tx).await?;
        tx.commit().await;
        Ok(())
    }
}
