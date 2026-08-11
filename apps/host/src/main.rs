use std::error::Error;

use opencarpanel_host::{HostConfig, bind_host};
use tracing::info;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .try_init()?;

    let running = bind_host(HostConfig::default()).await?;
    info!(
        http_address = %running.http_address(),
        udp_address = %running.udp_address(),
        "OpenCarpanel Host is ready"
    );
    tokio::signal::ctrl_c().await?;
    info!("shutdown requested");
    running.shutdown().await?;
    Ok(())
}
