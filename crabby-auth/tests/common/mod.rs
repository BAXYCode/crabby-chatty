use once_cell::sync::OnceCell;
use sqlx::{AssertSqlSafe, Connection, Executor, PgConnection, PgPool, SqlSafeStr, SqlStr};
use std::time::Duration;
use url::Url;
use uuid::Uuid;

static MIGRATOR_RAN: OnceCell<()> = OnceCell::new();

/// A per-test-run temporary database.
///
/// Why we create a fresh DB instead of truncating tables:
/// - avoids cross-test pollution
/// - keeps tests parallel-friendly
/// - migrations are applied once per temp DB
pub struct TestDb {
    pub pool: PgPool,
    admin_url: String,
    db_name: String,
}

impl TestDb {
    /// Creates a brand new database on the Postgres instance pointed to by DATABASE_URL.
    ///
    /// DATABASE_URL must point to a superuser (or a role allowed to CREATE DATABASE and CREATE EXTENSION).
    pub async fn new() -> eyre::Result<Self> {
        let base = std::env::var("DATABASE_URL")
            .or_else(|_| std::env::var("TEST_DATABASE_URL"))
            .map_err(|_| {
                eyre::eyre!("Set DATABASE_URL (or TEST_DATABASE_URL) for sqlx macros + tests")
            })?;

        let mut url = Url::parse(&base)?;
        // Use postgres DB for admin operations.
        url.set_path("/postgres");
        let admin_url = url.to_string();

        let db_name = format!("crabby_auth_test_{}", Uuid::new_v4().simple());

        // Create the database.
        let mut admin = PgConnection::connect(&admin_url).await?;
        let create_stmt = AssertSqlSafe(format!(r#"CREATE DATABASE {}"#, db_name)).into_sql_str();
        sqlx::query(create_stmt).execute(&mut admin).await?;

        // Connect to the new DB.
        let mut test_url = Url::parse(&base)?;
        test_url.set_path(&format!("/{}", db_name));
        let test_url = test_url.to_string();

        // Small retry loop in case Postgres needs a moment to accept the new DB.
        let mut last_err: Option<eyre::Report> = None;
        let pool = loop {
            match PgPool::connect(&test_url).await {
                Ok(p) => break p,
                Err(e) => {
                    last_err = Some(eyre::Report::new(e));
                    tokio::time::sleep(Duration::from_millis(150)).await;
                }
            }
        };

        // Run migrations.
        // Note: migrations create extensions + schema, so we need a privileged role.
        // sqlx::migrate! is compile-time embedded and does not require sqlx-cli.
        sqlx::migrate!("./migrations").run(&pool).await?;

        // Ensure we don't accidentally run the migrator multiple times *within* a test.
        // (Each TestDb runs migrations once; this is mainly to document intent.)
        let _ = MIGRATOR_RAN.set(());

        Ok(Self {
            pool,
            admin_url,
            db_name,
        })
    }

    pub async fn teardown(self) -> eyre::Result<()> {
        // Ensure all connections are dropped before DROP DATABASE.
        self.pool.close().await;

        let mut admin = PgConnection::connect(&self.admin_url).await?;
        // Postgres 13+: FORCE will terminate remaining connections.
        let drop_stmt = AssertSqlSafe(format!(
            r#"DROP DATABASE IF EXISTS {} WITH (FORCE)"#,
            self.db_name
        ))
        .into_sql_str();
        let _ = sqlx::query(drop_stmt).execute(&mut admin).await;
        Ok(())
    }
}
