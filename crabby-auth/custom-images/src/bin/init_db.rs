#![allow(unused_variables)]
use std::env;

use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::filter::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                // axum logs rejections from built-in extractors with the `axum::rejection`
                // target, at `TRACE` level. `axum::rejection=trace` enables showing those events
                "init_db=info".into()
            }),
        )
        .with(tracing_subscriber::fmt::layer().pretty())
        .init();
    info!(name:"Environment Variables", "fetching variables");
    let host = env::var("COCKROACH_HOST").expect("No Host provided");
    let port = env::var("COCKROACH_PORT").expect("No port provided");
    let _user = env::var("COCKROACH_USER").expect("OS user not provided");

    info!(name:"Environment Variables", "Connecting to cluster from {} at port number {}", host, port);
    let _insecure = env::var("COCKROACH_INSECURE");
    let _certs = env::var("COCKROACH_CERTS_DIR");

    let db_name = env::var("DB_NAME");
    let db_user = env::var("DB_USER");
    let db_pass = env::var("DB_PASS");

    let init = env::var("COCKROACH_INIT");

    if let Ok(init) = init {
        let initialized = std::process::Command::new("/cockroach")
            .args(["init", "--disable-cluster-name-verification"])
            .status();
        info!(name:"cluster Init", "Cluster initialization status{:?}", initialized.unwrap());
    }

    if let Ok(db_name) = db_name.clone() {
        let db = std::process::Command::new("/cockroach")
            .args([
                "sql",
                "--execute",
                std::format!("CREATE DATABASE IF NOT EXISTS {}", db_name).as_str(),
            ])
            .status()
            .expect("could not create Database");
        info!(name: "db initialization", "Database initialization status: {:?}", db);
    }

    if let Ok(username) = db_user {
        let user_create = std::process::Command::new("/cockroach")
            .args([
                "sql",
                "--execute",
                std::format!(
                    "CREATE USER IF NOT EXISTS {} WITH PASSWORD {}",
                    username,
                    db_pass.unwrap_or_else(|_| { "NULL".to_owned() })
                )
                .as_str(),
            ])
            .status()
            .expect("could not create user");
        info!(name: "User creation", "User creation status: {:?}", user_create);
        let privileges = std::process::Command::new("/cockroach")
            .args([
                "sql",
                "--execute",
                std::format!(
                    "GRANT ALL ON DATABASE {} TO {}",
                    db_name.expect("no db name"),
                    username
                )
                .as_str(),
                "--execute",
                std::format!("GRANT admin TO {}", username).as_str(),
            ])
            .status()
            .expect("could not grant permissions");
        info!(name:"permissions and roles", "Granting permissions and roles status: {:?}",privileges)
    }

    Ok(())
}
