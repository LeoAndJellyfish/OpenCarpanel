use std::{error::Error, time::Duration};

use opencarpanel_config::SettingsRepository;
use opencarpanel_host::{
    AdapterSelection, HostConfig, InstanceGuard, InstanceMode, bind_host, default_data_directory,
};
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("OpenCarpanel 启动失败：{error}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .try_init()?;

    let _instance = InstanceGuard::acquire(InstanceMode::Headless)?;
    let data_directory = default_data_directory();
    let loaded = SettingsRepository::new(&data_directory).load()?;
    if let Some(path) = loaded.quarantined_path.as_ref() {
        warn!(
            path = %path.display(),
            reset_to_defaults = loaded.reset_to_defaults,
            "recovered invalid application settings"
        );
    }
    let mut config = HostConfig::from_settings(&loaded.settings.host, data_directory)?;
    apply_environment_overrides(&mut config)?;

    let running = bind_host(config).await?;
    info!(
        http_address = %running.http_address(),
        udp_address = %running.udp_address(),
        adapter_selection = %running.state().adapter_selection(),
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
        "游戏遥测 UDP 目标端口：{}\n选择游戏可在桌面控制中心设置，或使用 OPENCARPANEL_GAME=auto|f1-24|f1-25|ets2|ats（当前：{}）。\n按 Ctrl+C 退出。\n",
        running.udp_address().port(),
        running.state().adapter_selection(),
    );
    tokio::signal::ctrl_c().await?;
    info!("shutdown requested");
    running.shutdown().await?;
    Ok(())
}

fn apply_environment_overrides(
    config: &mut HostConfig,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let Ok(address) = std::env::var("OPENCARPANEL_HTTP_BIND") {
        config.http_address = address.parse()?;
    }
    if let Ok(address) = std::env::var("OPENCARPANEL_UDP_BIND") {
        config.udp_address = address.parse()?;
    }
    if let Ok(selection) = std::env::var("OPENCARPANEL_GAME") {
        config.adapter_selection = selection.parse::<AdapterSelection>()?;
    }
    if let Ok(snapshot_hz) = std::env::var("OPENCARPANEL_SNAPSHOT_HZ") {
        let snapshot_hz = snapshot_hz.parse::<u16>()?;
        if !matches!(snapshot_hz, 20 | 30 | 60) {
            return Err(format!(
                "OPENCARPANEL_SNAPSHOT_HZ must be 20, 30, or 60; got {snapshot_hz}"
            )
            .into());
        }
        config.snapshot_hz_limit = snapshot_hz;
    }
    Ok(())
}
