mod proto {
    tonic::include_proto!("weve_esi_proto");
}
mod esi_client;
mod service;
mod config;
mod cache;
mod error;
mod json;
mod time;
mod env;

type RefreshToken = String;
type MarketName = String;
type LocationId = i64;
type RegionId = i32;
type TypeId = i32;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let service = env::service_from_env().unwrap();

    service.serve().await.unwrap();

    Ok(())
}
