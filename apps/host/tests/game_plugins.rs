use std::{collections::BTreeSet, error::Error, fs, net::SocketAddr};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use opensimdash_game_plugin_api::{
    GAME_PLUGIN_ABI_VERSION, GAME_PLUGIN_PACKAGE_VERSION, GamePluginPackage, PluginRuntime,
    PluginSource, parse_manifest,
};
use opensimdash_game_plugin_runtime::install_package;
use opensimdash_host::{AdapterSelection, HostConfig, bind_host};
use sha2::{Digest as _, Sha256};
use tempfile::tempdir;
use tokio::net::UdpSocket;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn installed_wasm_plugin_is_discovered_selected_and_decodes_udp() -> Result<(), Box<dyn Error>>
{
    let temporary = tempdir()?;
    let response = r#"{"schemaVersion":1,"updates":[{"receivedAt":0,"vehicle":{"speedMps":42.5}}],"events":[]}"#;
    let module = wat::parse_str(format!(
        r#"(module
          (memory (export "memory") 6 64)
          (data (i32.const 65536) "{}")
          (func (export "osd_plugin_abi_version") (result i32) i32.const 1)
          (func (export "osd_input_ptr") (result i32) i32.const 0)
          (func (export "osd_input_capacity") (result i32) i32.const 65536)
          (func (export "osd_output_ptr") (result i32) i32.const 65536)
          (func (export "osd_output_capacity") (result i32) i32.const 262144)
          (func (export "osd_decode") (param i32 i64) (result i32) i32.const {})
        )"#,
        wat_escape(response),
        response.len(),
    ))?;
    let mut manifest =
        parse_manifest(include_bytes!("../../../plugins/games/f1-24/manifest.json"))?;
    manifest.id = "community-sim".to_owned();
    manifest.name = "Community Sim".to_owned();
    manifest.version = "1.0.0".to_owned();
    manifest.publisher = "OpenSimDash test".to_owned();
    manifest.runtime = PluginRuntime::Wasm {
        abi_version: GAME_PLUGIN_ABI_VERSION,
        module: "decoder.wasm".to_owned(),
        sha256: format!("{:x}", Sha256::digest(&module)),
    };
    let package = GamePluginPackage {
        package_version: GAME_PLUGIN_PACKAGE_VERSION,
        manifest,
        module_base64: STANDARD.encode(&module),
    };
    let package_path = temporary.path().join("community-sim.osd-plugin");
    fs::write(&package_path, serde_json::to_vec(&package)?)?;
    let plugins_root = temporary.path().join("game-plugins");
    let receipt = install_package(&plugins_root, &package_path, &BTreeSet::new())?;
    assert_eq!(receipt.id, "community-sim");

    let loopback_zero: SocketAddr = "127.0.0.1:0".parse()?;
    let running = bind_host(HostConfig {
        http_address: loopback_zero,
        udp_address: loopback_zero,
        adapter_selection: "community-sim".parse::<AdapterSelection>()?,
        snapshot_hz_limit: 60,
        data_directory: temporary.path().to_path_buf(),
    })
    .await?;
    let supported = running
        .state()
        .supported_adapters()
        .iter()
        .find(|adapter| adapter.id() == "community-sim")
        .ok_or("installed plugin missing from Host metadata")?;
    assert_eq!(supported.metadata().source, PluginSource::Installed);

    let mut snapshots = running.state().subscribe_snapshots();
    let sender = UdpSocket::bind(loopback_zero).await?;
    sender
        .send_to(b"plugin-packet", running.udp_address())
        .await?;
    tokio::time::timeout(std::time::Duration::from_secs(2), snapshots.changed()).await??;
    let snapshot = snapshots.borrow().clone();
    assert_eq!(snapshot.meta.game_id.as_deref(), Some("community-sim"));
    assert_eq!(snapshot.vehicle.speed_mps, Some(42.5));
    assert_eq!(running.state().metrics().packets_recognized, 1);

    drop(snapshots);
    running.shutdown().await?;
    Ok(())
}

fn wat_escape(value: &str) -> String {
    value
        .bytes()
        .map(|byte| match byte {
            b'"' => "\\22".to_owned(),
            b'\\' => "\\5c".to_owned(),
            0x20..=0x7e => char::from(byte).to_string(),
            _ => format!("\\{byte:02x}"),
        })
        .collect()
}
