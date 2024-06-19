use anyhow::Result;
use std::env;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn main() -> Result<()> {
    println!("test");
    // tracing
    let _sub: () = tracing_subscriber::registry()
        .with(
            tracing_subscriber::filter::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                // axum logs rejections from built-in extractors with the `axum::rejection`
                // target, at `TRACE` level. `axum::rejection=trace` enables showing those events
                "certs_gen=info".into()
            }),
        )
        .with(tracing_subscriber::fmt::layer().pretty())
        .init();
    // environment variables used by cockroach-cli
    info!(name:"Environment Variables", "fetching  necessary environment variables");
    let username = env::var("CLIENT_USERNAME").expect("No client username provided");
    let alt_names = env::var("NODE_ALTERNATIVE_NAMES").expect("No alternative names provided");
    let mut usernames = Vec::new();
    usernames.push("root");
    let certs_dir = "/.cockroach-certs";
    let ca_key = "/.cockroach-key/ca.key";
    if username != "root" {
        usernames.push(username.as_str());
    }

    let alt_names = alt_names.trim().split(" ").collect::<Vec<&str>>();
    println!("alt names: {:?}", alt_names);

    info!("Creating Certificate Authority");
    let ca = std::process::Command::new("/cockroach")
        .args([
            "cert",
            "create-ca",
            "--certs-dir",
            certs_dir,
            "--ca-key",
            ca_key,
            // "--overwrite",
        ])
        .status()
        .expect("could not create Certificate Authority");
    info!("Certificate Authority creation {}", ca);

    info!("Creating Client certificates");
    for user in usernames {
        std::process::Command::new("/cockroach")
            .args([
                "cert",
                "create-client",
                user,
                "--certs-dir",
                certs_dir,
                "--ca-key",
                ca_key,
                // "--overwrite",
            ])
            .status()
            .expect("could not create certificate for certain user");
    }

    info!("Creating Node Certificates");
    let mut args = vec!["cert", "create-node"];
    alt_names.iter().map(|name| args.push(name)).max();
    args.extend(vec![
        "--certs-dir",
        certs_dir,
        "--ca-key",
        ca_key,
        // "--overwrite",
    ]);
    let _node = std::process::Command::new("/cockroach")
        .args(args)
        .status()
        .expect("could not generate certificates for node");
    println!("test2");

    Ok(())
}
