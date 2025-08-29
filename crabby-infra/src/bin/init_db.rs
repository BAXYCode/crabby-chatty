use anyhow::{Context, Error, Result};
use backon::{BackoffBuilder, BlockingRetryable, ExponentialBuilder};
use reqwest::redirect::Policy;
use std::env;
use tracing::{info, instrument, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn main() -> Result<()> {
    // logging
    let _sub: () = tracing_subscriber::registry()
        .with(
            tracing_subscriber::filter::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                // axum logs rejections from built-in extractors with the `axum::rejection`
                // target, at `TRACE` level. `axum::rejection=trace` enables showing those events
                "init_db=info".into()
            }),
        )
        .with(tracing_subscriber::fmt::layer().pretty())
        .init();
    // environment variables used by cockroach-cli
    info!(name:"Environment Variables", "fetching  necessary environment variables");
    let host = env::var("COCKROACH_HOST").expect("No Host provided");
    let port = env::var("COCKROACH_PORT").expect("No port provided");
    // let _user = env::var("COCKROACH_USER").expect("OS user not provided");
    // environment variables used by this tool to contact cockroach cluster

    info!(name:"Environment Variables", "Connecting to cluster from {} at port number {}", host, port);
    let insecure =
        env::var("COCKROACH_INSECURE").expect("Specify if cluster will be insecure or not");
    let certs = env::var("COCKROACH_CERTS_DIR");
    // Initialization info to setup db
    // let db_name = env::var("DATABASE_NAME");
    // let db_user = env::var("DATABASE_USER");
    // let db_pass = env::var("DATABASE_PASS");

    let healthcheck = env::var("HEALTHCHECK").expect("Healthcheck url is needed");

    // Environment variable specifying wether to initialize the cluster
    let init = env::var("COCKROACH_INIT");
    let mut certs_dir = String::new();

    if let Ok(init) = init {
        if insecure.as_str() == "false" {
            certs_dir = format!(
                "--certs-dir={}",
                certs.expect("certificate directory is needed")
            );
            info!(
                "Secure initialization with directory {} for the certificates",
                certs_dir
            );
            let initialized = std::process::Command::new("/cockroach")
                .args([
                    "init",
                    certs_dir.as_str(),
                    "--disable-cluster-name-verification",
                ])
                .status();

            info!(name:"cluster Init", "Secure cluster initialization status {:?}", initialized.unwrap());
        } else {
            let initialized = std::process::Command::new("/cockroach")
                .args(["init", "--disable-cluster-name-verification"])
                .status();
            info!(name:"cluster Init", "Insecure cluster initialization status {:?}", initialized.unwrap());
        }
    }

    wait_ready(healthcheck).expect("failed to get ready");
    // if let Ok(db_name) = db_name.clone() {
    //     if insecure.as_str() == "false" {
    //         info!(name: "db initialization", "Initializing Database in Secure mode");
    //         let db = std::process::Command::new("/cockroach")
    //             .args([
    //                 "sql",
    //                 certs_dir.as_str(),
    //                 "--execute",
    //                 std::format!("CREATE DATABASE IF NOT EXISTS {}", db_name).as_str(),
    //             ])
    //             .status()
    //             .expect("could not create Database");
    //     } else {
    //         info!(name: "db initialization", "Initializing Database in Insecure mode");
    //         let db = std::process::Command::new("/cockroach")
    //             .args([
    //                 "sql",
    //                 "--execute",
    //                 std::format!("CREATE DATABASE IF NOT EXISTS {}", db_name).as_str(),
    //             ])
    //             .status()
    //             .expect("could not create Database");
    //     }
    // } else {
    //     warn!("No database name provided, no new database will be made")
    // }
    // if let Ok(username) = db_user {
    //     if insecure.as_str() == "false" {
    //         let user_create = std::process::Command::new("/cockroach")
    //             .args([
    //                 "sql",
    //                 certs_dir.as_str(),
    //                 "--execute",
    //                 std::format!(
    //                     "CREATE USER IF NOT EXISTS {} WITH PASSWORD {}",
    //                     username,
    //                     db_pass.unwrap_or_else(|_| { "NULL".to_owned() })
    //                 )
    //                 .as_str(),
    //             ])
    //             .status()
    //             .expect("could not create user");
    //
    //         info!(name: "User creation", "User creation {:?} in Secure mode", user_create);
    //         let privileges = std::process::Command::new("/cockroach")
    //             .args([
    //                 "sql",
    //                 certs_dir.as_str(),
    //                 "--execute",
    //                 std::format!(
    //                     "GRANT ALL ON DATABASE {} TO {}",
    //                     db_name.expect("no db name"),
    //                     username
    //                 )
    //                 .as_str(),
    //                 "--execute",
    //                 std::format!("GRANT admin TO {}", username).as_str(),
    //             ])
    //             .status()
    //             .expect("could not grant permissions");
    //         info!(name:"permissions and roles", "Granting permissions and roles {:?} in Secure mode",privileges)
    //     } else {
    //         let user_create = std::process::Command::new("/cockroach")
    //             .args([
    //                 "sql",
    //                 "--execute",
    //                 std::format!(
    //                     "CREATE USER IF NOT EXISTS {} WITH PASSWORD {}",
    //                     username,
    //                     db_pass.unwrap_or_else(|_| { "NULL".to_owned() })
    //                 )
    //                 .as_str(),
    //             ])
    //             .status()
    //             .expect("could not create user");
    //
    //         info!(name: "User creation", "User creation {:?} in Insecure mode", user_create);
    //         let privileges = std::process::Command::new("/cockroach")
    //             .args([
    //                 "sql",
    //                 "--execute",
    //                 std::format!(
    //                     "GRANT ALL ON DATABASE {} TO {}",
    //                     db_name.expect("no db name"),
    //                     username
    //                 )
    //                 .as_str(),
    //                 "--execute",
    //                 std::format!("GRANT admin TO {}", username).as_str(),
    //             ])
    //             .status()
    //             .expect("could not grant permissions");
    //         info!(name:"permissions and roles", "Granting permissions and roles {:?} in Insecure mode",privileges)
    //     }
    // } else {
    //     warn!("No database username provided, no new user will be created and no roles will be granted");
    // }
    Ok(())
}

#[instrument]
pub fn wait_ready(healthcheck: String) -> Result<()> {
    info!(name:"Node Boot up", "waiting on Node activation");
    info!("healthcheck endpoint: {}", healthcheck);
    let op = || {
        let url = reqwest::Url::parse(&healthcheck)
            .map_err(Error::new)
            .context("parsing error, make sure url is in appropriate format")?;
        let builder = reqwest::blocking::ClientBuilder::new();
        let client = builder
            //FIX: very bad, find a better way to do this!!!!!!
            .danger_accept_invalid_certs(true)
            .redirect(Policy::limited(10000))
            .build()
            .expect("could not build reqwest client");
        let ready = client.get(url).send()?.error_for_status()?;
        Ok(ready)
    };
    // let backoff_builder = ExponentialBuilder::default()
    //     .with_max_times(10usize)
    //     .build();
    // Try to contact the cluster and wait for it to spin-up if it is offline
    let retry = op
        .retry(&ExponentialBuilder::default().with_max_times(10usize))
        .when(|e: &anyhow::Error| e.to_string() != "parsing error")
        .notify(|err, duration| {
            warn!(
                "Failed with {:?}, will retry after {:?}",
                err.source(),
                duration
            );
        })
        .call()?;
    info!("Response status: {:?}", retry.status());

    Ok(())
}
