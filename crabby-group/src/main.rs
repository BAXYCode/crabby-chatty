use std::sync::Arc;

use crabby_group::{
    api::StorageState, database::repo::PgRepo, grpc::GroupServiceImpl,
};
use sqlx::postgres::PgPoolOptions;
use tonic::{service::Routes, transport::Server};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await?;

    let http_addr = std::env::var("HTTP_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:8080".to_string());
    // let grpc_addr = std::env::var("GRPC_ADDR")
    //     .unwrap_or_else(|_| "0.0.0.0:50051".to_string());

    let state = StorageState {
        store: Arc::new(PgRepo::new(pool.clone())),
    };
    //create gRPC routes
    let mut builder = Routes::builder();
    builder.add_service(GroupServiceImpl::new(pool).into_server());
    let grpc = builder.routes().into_axum_router().with_state(());
    //create HTTP routes
    let (http, _api) = crabby_group::api::router().split_for_parts();
    //merge to serve on same endpoint
    let http = http.with_state(state).merge(grpc);
    let server =
        axum::serve(tokio::net::TcpListener::bind(&http_addr).await?, http)
            .into_future();

    println!("HTTP listening on {http_addr}");
    // println!("gRPC listening on {grpc_addr}");

    tokio::try_join!(async move { server.await.map_err(eyre::Error::from) })?;

    Ok(())
}


