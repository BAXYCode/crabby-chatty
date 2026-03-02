//! Simple request-per-second (RPS) smoke load tests.
//!
//! These are marked `#[ignore]` because they require a running auth service
//! (and its database) and they can be noisy/flaky in CI.
//!
//! Run one like:
//!   cargo test -p crabby-auth rps_login -- --ignored --nocapture
//!
//! Configure via env vars:
//!   AUTH_ADDR=http://127.0.0.1:6769
//!   CONCURRENCY=64
//!   DURATION_SECS=10
use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use eyre::Result;
use fake::{Fake, faker::internet::en::Username};
use once_cell::sync::Lazy;
use tonic::{Request, transport::Channel};

use crabby_auth::authenticate::auth::{
    LoginRequest, RefreshRequest, RegisterRequest, authenticate_client::AuthenticateClient,
};
use rand::{Rng, distr::Alphanumeric};

static RUN_ID: Lazy<String> = Lazy::new(|| {
    // short, cheap, unique enough for test runs
    let s: String = rand::rng()
        .sample_iter(&Alphanumeric)
        .take(10)
        .map(char::from)
        .collect();
    s
});

#[derive(Clone)]
struct Creds {
    email: String,
    username: String,
    password: String,
}

impl Creds {
    fn new_deterministic(i: u64) -> Self {
        let username = format!("rps_{}_user{}", &*RUN_ID, i);
        let email = format!("rps_{}_user{}@example.test", &*RUN_ID, i);

        Self {
            username,
            email,
            password: "password123!".to_string(),
        }
    }

    fn register_req(&self) -> RegisterRequest {
        RegisterRequest {
            email: self.email.clone(),
            username: self.username.clone(),
            password: self.password.clone(),
        }
    }

    fn login_req(&self) -> LoginRequest {
        LoginRequest {
            username: self.username.clone(),
            password: self.password.clone(),
        }
    }
}
fn data_pool_size() -> usize {
    // Default big enough for register tests:
    // requests ~= concurrency * duration_secs * expected_rps
    // Your current ~80 RPS at 10s*64 => ~50k requests, so 200k is safe.
    std::env::var("TESTDATA_POOL_SIZE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(500_000)
}

static CREDS_POOL: Lazy<Vec<Creds>> = Lazy::new(|| {
    let n = data_pool_size();
    let mut v = Vec::with_capacity(n);
    for i in 0..n as u64 {
        v.push(Creds::new_deterministic(i));
    }
    v
});

static CREDS_IDX: AtomicU64 = AtomicU64::new(0);

fn next_creds() -> Creds {
    let i = CREDS_IDX.fetch_add(1, Ordering::Relaxed) as usize;
    CREDS_POOL
        .get(i)
        .unwrap_or_else(|| {
            panic!("Ran out of pre-generated creds (need > {i}). Increase TESTDATA_POOL_SIZE.")
        })
        .clone()
}

async fn client() -> Result<AuthenticateClient<Channel>> {
    let addr = std::env::var("AUTH_ADDR").unwrap_or_else(|_| "http://127.0.0.1:6769".to_string());
    Ok(AuthenticateClient::connect(addr).await?)
}

fn concurrency() -> usize {
    std::env::var("CONCURRENCY")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(32)
}

fn duration() -> Duration {
    let secs: u64 = std::env::var("DURATION_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(10);
    Duration::from_secs(secs)
}

fn refresh_request_with_auth(refresh_token: &str) -> Request<RefreshRequest> {
    let mut req = Request::new(RefreshRequest {
        refresh: String::new(),
        user_id: String::new(),
    });
    let header_val = format!("Bearer {refresh_token}");
    req.metadata_mut().insert(
        "authorization",
        header_val.parse().expect("valid metadata value"),
    );
    req
}

async fn run_rps<F, Fut>(name: &str, make_call: F) -> Result<()>
where
    // 👇 Clone is the key fix here
    F: Fn() -> Fut + Send + Sync + Clone + 'static,
    Fut: std::future::Future<Output = bool> + Send + 'static,
{
    let c = concurrency();
    let dur = duration();
    let end = Instant::now() + dur;

    let ok = Arc::new(AtomicU64::new(0));
    let err = Arc::new(AtomicU64::new(0));

    let mut handles = Vec::with_capacity(c);
    for _ in 0..c {
        let end = end;
        let make_call = make_call.clone(); // 👈 per-worker clone
        let ok = Arc::clone(&ok);
        let err = Arc::clone(&err);

        let handle = tokio::spawn(async move {
            while Instant::now() < end {
                if make_call().await {
                    ok.fetch_add(1, Ordering::Relaxed);
                } else {
                    err.fetch_add(1, Ordering::Relaxed);
                }
            }
        });

        handles.push(handle);
    }

    for h in handles {
        let _ = h.await;
    }

    let ok = ok.load(Ordering::Relaxed);
    let err = err.load(Ordering::Relaxed);
    let total = ok + err;
    let rps = (total as f64) / dur.as_secs_f64();

    println!("\n=== {name} ===");
    println!("duration: {:?}", dur);
    println!("concurrency: {c}");
    println!("ok: {ok}  err: {err}  total: {total}");
    println!("RPS: {:.2}", rps);

    Ok(())
}

/// Measures how many Login RPCs per second the server can handle.
#[tokio::test]
#[ignore]
async fn rps_login() -> Result<()> {
    // Prep: create a pool of users once so the test measures login, not register.
    let c = concurrency();
    let mut creds = Vec::with_capacity(c);
    for _ in 0..c {
        creds.push(next_creds());
    }

    // Register all users (sequential is fine; this is just setup).
    {
        let mut cli = client().await?;
        for u in &creds {
            let _ = cli.register(Request::new(u.register_req())).await?;
        }
    }

    // One client per task to avoid lock contention in the channel.
    let mut clients = Vec::with_capacity(c);
    for _ in 0..c {
        clients.push(client().await?);
    }

    // Round-robin across the pre-registered users.
    let idx = std::sync::Arc::new(AtomicU64::new(0));
    let clients = std::sync::Arc::new(clients);
    let creds = std::sync::Arc::new(creds);

    run_rps("login", move || {
        let idx = idx.clone();
        let clients = clients.clone();
        let creds = creds.clone();
        async move {
            let i = idx.fetch_add(1, Ordering::Relaxed) as usize;
            let mut cli = clients.get(i % clients.len()).expect("client").clone();
            let u = creds.get(i % creds.len()).expect("creds").clone();
            cli.login(Request::new(u.login_req())).await.is_ok()
        }
    })
    .await
}

/// Measures how many Register RPCs per second the server can handle.
#[tokio::test]
#[ignore]
async fn rps_register() -> Result<()> {
    let c = concurrency();
    let mut clients = Vec::with_capacity(c);
    for _ in 0..c {
        clients.push(client().await?);
    }

    let idx = std::sync::Arc::new(AtomicU64::new(0));
    let clients = std::sync::Arc::new(clients);

    run_rps("register", move || {
        let idx = idx.clone();
        let clients = clients.clone();
        async move {
            let i = idx.fetch_add(1, Ordering::Relaxed) as usize;
            let mut cli = clients.get(i % clients.len()).expect("client").clone();
            let u = next_creds();
            cli.register(Request::new(u.register_req())).await.is_ok()
        }
    })
    .await
}

/// Measures how many Refresh RPCs per second the server can handle.
///
/// Each worker registers once and then reuses its own refresh token.
#[tokio::test]
#[ignore]
async fn rps_refresh() -> Result<()> {
    let c = concurrency();

    // Setup: create one user per worker and capture its refresh token.
    let mut refresh_tokens = Vec::with_capacity(c);
    {
        let mut cli = client().await?;
        for _ in 0..c {
            let u = next_creds();
            let res = cli
                .register(Request::new(u.register_req()))
                .await?
                .into_inner();
            let token = res
                .response
                .as_ref()
                .expect("register success")
                .refresh
                .clone();
            refresh_tokens.push(token);
        }
    }

    let mut clients = Vec::with_capacity(c);
    for _ in 0..c {
        clients.push(client().await?);
    }

    let idx = std::sync::Arc::new(AtomicU64::new(0));
    let clients = std::sync::Arc::new(clients);
    let refresh_tokens = std::sync::Arc::new(refresh_tokens);

    run_rps("refresh", move || {
        let idx = idx.clone();
        let clients = clients.clone();
        let refresh_tokens = refresh_tokens.clone();
        async move {
            let i = idx.fetch_add(1, Ordering::Relaxed) as usize;
            let mut cli = clients.get(i % clients.len()).expect("client").clone();
            let token = refresh_tokens
                .get(i % refresh_tokens.len())
                .expect("refresh token")
                .clone();
            cli.refresh(refresh_request_with_auth(&token)).await.is_ok()
        }
    })
    .await
}
