mod common;

use crabby_auth::paseto::keys_repo::{PasetoKeyRepo, PostgresKeyRepo};
use crabby_auth::domain::models::RefreshTokenRow;
use pasetors::keys::{AsymmetricKeyPair, Generate, SymmetricKey};
use pasetors::paserk::{FormatAsPaserk, Id};
use pasetors::version4::V4;
use std::sync::Arc;
use uuid::Uuid;

#[tokio::test]
async fn store_and_fetch_public_and_local_keys_by_kid() -> eyre::Result<()> {
    let db = common::TestDb::new().await?;
    let repo = PostgresKeyRepo {
        conn: Arc::new(db.pool.clone()),
    };

    // Public (asymmetric) key
    let kp = AsymmetricKeyPair::<V4>::generate().unwrap();
    let kid_pub = {
        let id = Id::from(&kp.public);
        let mut s = String::new();
        id.fmt(&mut s);
        s
    };

    repo.store_public_key(kp.public.clone()).await?;
    let fetched_pub = repo.fetch_public_key(kid_pub.clone()).await?;
    assert_eq!(fetched_pub.as_bytes(), kp.public.as_bytes());

    // Local-wrap (symmetric) key
    let sk = SymmetricKey::<V4>::generate().unwrap();
    let kid_local = {
        let id = Id::from(&sk);
        let mut s = String::new();
        id.fmt(&mut s);
        s
    };
    repo.store_local_key(sk.clone()).await?;
    let fetched_local = repo.fetch_local_key(kid_local.clone()).await?;
    assert_eq!(fetched_local.as_bytes(), sk.as_bytes());

    db.teardown().await?;
    Ok(())
}

#[tokio::test]
async fn store_fetch_and_rotate_refresh_token_metadata() -> eyre::Result<()> {
    let db = common::TestDb::new().await?;

    // Need a user because refresh_token.user_id references auth_user.
    let user_id: Uuid = sqlx::query_scalar(
        r#"INSERT INTO validation.auth_user (email, username, password_hash)
           VALUES ($1, $2, $3)
           RETURNING user_id"#,
    )
    .bind("rt@example.com")
    .bind("rt_user")
    .bind("hash")
    .fetch_one(&db.pool)
    .await?;

    let repo = PostgresKeyRepo {
        conn: Arc::new(db.pool.clone()),
    };

    let old_jti = Uuid::new_v4();
    let old_hash = vec![1u8; 32];
    let now = chrono::Utc::now();
    let row = RefreshTokenRow {
        user_id,
        token_jti: old_jti,
        token_hash: old_hash.clone(),
        issued_at: now,
        expires_at: now + chrono::Duration::days(14),
    };

    repo.store_refresh_info(&row).await?;
    let fetched = repo.fetch_refresh_token(&old_jti, &user_id).await?;
    assert_eq!(fetched.user_id, user_id);
    assert_eq!(fetched.token_jti, old_jti);
    assert_eq!(fetched.token_hash, old_hash);

    let new_jti = Uuid::new_v4();
    let new_hash = vec![2u8; 32];
    let new_row = RefreshTokenRow {
        user_id,
        token_jti: new_jti,
        token_hash: new_hash.clone(),
        issued_at: now,
        expires_at: now + chrono::Duration::days(21),
    };

    // Successful rotation
    let rotated = repo
        .rotate_refresh_info(&user_id, &old_jti, &old_hash, &new_row)
        .await?;
    assert!(rotated);

    let fetched2 = repo.fetch_refresh_token(&new_jti, &user_id).await?;
    assert_eq!(fetched2.token_hash, new_hash);

    // Rotation with wrong old hash must fail
    let rotated2 = repo
        .rotate_refresh_info(&user_id, &new_jti, &[9u8; 32], &row)
        .await?;
    assert!(!rotated2);

    db.teardown().await?;
    Ok(())
}
