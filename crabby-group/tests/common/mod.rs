use sqlx::{AssertSqlSafe, Connection, PgConnection, PgPool, SqlSafeStr};
use testcontainers_modules::{
    postgres::Postgres,
    testcontainers::{runners::AsyncRunner, ImageExt},
};
use tokio::sync::OnceCell;
use url::Url;
use uuid::Uuid;

static PG_URL: OnceCell<String> = OnceCell::const_new();

async fn base_url() -> &'static str {
    PG_URL
        .get_or_init(|| async {
            let container = Box::leak(Box::new(
                Postgres::default()
                    .with_tag("18")
                    .start()
                    .await
                    .expect("failed to start postgres 18 container"),
            ));
            let host = container.get_host().await.unwrap();
            let port = container.get_host_port_ipv4(5432).await.unwrap();
            format!("postgresql://postgres:postgres@{host}:{port}/postgres")
        })
        .await
}

pub struct TestDb {
    pub pool: PgPool,
    admin_url: String,
    db_name: String,
}

impl TestDb {
    pub async fn new() -> eyre::Result<Self> {
        let base = base_url().await;
        let mut url = Url::parse(base)?;
        url.set_path("/postgres");
        let admin_url = url.to_string();
        let db_name = format!("crabby_group_test_{}", Uuid::new_v4().simple());

        let mut admin = PgConnection::connect(&admin_url).await?;
        sqlx::query(AssertSqlSafe(format!("CREATE DATABASE {db_name}")).into_sql_str())
            .execute(&mut admin)
            .await?;

        let mut test_url = Url::parse(base)?;
        test_url.set_path(&format!("/{db_name}"));
        let pool = PgPool::connect(test_url.as_str()).await?;

        sqlx::migrate!("./migrations").run(&pool).await?;

        Ok(Self {
            pool,
            admin_url,
            db_name,
        })
    }

    /// Apply a fixture. Pass a `&'static str` (e.g. via `include_str!`).
    pub async fn with_fixture(self, sql: &'static str) -> eyre::Result<Self> {
        sqlx::raw_sql(sql).execute(&self.pool).await?;
        Ok(self)
    }

    pub async fn teardown(self) -> eyre::Result<()> {
        self.pool.close().await;
        let mut admin = PgConnection::connect(&self.admin_url).await?;
        sqlx::query(
            AssertSqlSafe(format!("DROP DATABASE IF EXISTS {} WITH (FORCE)", self.db_name))
                .into_sql_str(),
        )
        .execute(&mut admin)
        .await?;
        Ok(())
    }
}
