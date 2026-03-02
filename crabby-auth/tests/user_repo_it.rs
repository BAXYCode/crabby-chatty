mod common;

use crabby_auth::users::user_repo::{PostgresUserRepo, UserRepo};
use crabby_auth::domain::models::{EmailAddress, Password, RegisterRequestData, Username};
use std::sync::Arc;

#[tokio::test]
async fn register_and_fetch_user_roundtrip() -> eyre::Result<()> {
    let db = common::TestDb::new().await?;
    let repo = PostgresUserRepo {
        conn: Arc::new(db.pool.clone()),
    };

    let req = RegisterRequestData {
        username: Username::from("benjamin".to_string()),
        email: EmailAddress::from("ben@example.com".to_string()),
        password: Password::from("$argon2id$v=19$m=16,t=2,p=1$ZmFrZXNhbHQ$ZmFrZWhhc2g".to_string()),
    };

    let created = repo.register_user(req).await?;
    assert_eq!(created.username.to_lowercase(), "benjamin");

    let by_id = repo.get_user_from_id(created.user_id).await?;
    assert_eq!(by_id.user_id, created.user_id);
    assert_eq!(by_id.username.username.to_lowercase(), "benjamin");
    assert_eq!(by_id.email.email.to_lowercase(), "ben@example.com");

    let by_username = repo.get_user_from_username("benjamin").await?;
    assert_eq!(by_username.user_id, created.user_id);

    db.teardown().await?;
    Ok(())
}

#[tokio::test]
async fn register_enforces_unique_username_and_email() -> eyre::Result<()> {
    let db = common::TestDb::new().await?;
    let repo = PostgresUserRepo {
        conn: Arc::new(db.pool.clone()),
    };

    let req1 = RegisterRequestData {
        username: Username::from("uniqueuser".to_string()),
        email: EmailAddress::from("unique@example.com".to_string()),
        password: Password::from("hash1".to_string()),
    };
    let _ = repo.register_user(req1).await?;

    // Same username should fail.
    let req2 = RegisterRequestData {
        username: Username::from("uniqueuser".to_string()),
        email: EmailAddress::from("other@example.com".to_string()),
        password: Password::from("hash2".to_string()),
    };
    assert!(repo.register_user(req2).await.is_err());

    // Same email should fail.
    let req3 = RegisterRequestData {
        username: Username::from("otheruser".to_string()),
        email: EmailAddress::from("unique@example.com".to_string()),
        password: Password::from("hash3".to_string()),
    };
    assert!(repo.register_user(req3).await.is_err());

    db.teardown().await?;
    Ok(())
}
