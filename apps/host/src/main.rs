use std::{error::Error, time::Duration};

use opencarpanel_host::{HostConfig, bind_host};
use tracing::{info, warn};
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
    let pairing_token = running
        .issue_pairing_token(Duration::from_secs(15 * 60))
        .await?;
    let pairing_url = opencarpanel_host::pairing_url(running.http_address(), &pairing_token);
    let dashboard_url = pairing_url
        .split_once("/#")
        .map_or(pairing_url.as_str(), |(base, _fragment)| base);
    println!("\nOpenCarpanel 已启动。请让手机/iPad 与电脑连接同一局域网。\n");
    println!("配对地址（15 分钟内一次有效）：\n{pairing_url}\n");
    println!("扫描二维码：\n");
    match opencarpanel_host::terminal_qr(&pairing_url) {
        Ok(qr) => println!("{qr}"),
        Err(error) => warn!(%error, "could not render terminal pairing QR code"),
    }
    println!("\n配对后可打开编辑器：{dashboard_url}/edit");
    println!("本机诊断：{dashboard_url}/api/v1/diagnostics\n");
    println!(
        "F1 24 UDP 目标端口：{}\n按 Ctrl+C 退出。\n",
        running.udp_address().port()
    );
    tokio::signal::ctrl_c().await?;
    info!("shutdown requested");
    running.shutdown().await?;
    Ok(())
}
