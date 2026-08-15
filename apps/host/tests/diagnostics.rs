use std::{error::Error, io, time::Duration};

use opensimdash_host::spawn_host;
use serde_json::Value;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream, UdpSocket},
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn diagnostics_are_local_bounded_and_sanitized() -> Result<(), Box<dyn Error>> {
    let running = spawn_host(
        TcpListener::bind("127.0.0.1:0").await?,
        UdpSocket::bind("127.0.0.1:0").await?,
    )?;
    let sender = UdpSocket::bind("127.0.0.1:0").await?;
    sender.send_to(&[0], running.udp_address()).await?;

    let body = wait_for_packet_diagnostics(running.http_address()).await?;
    assert_eq!(body["status"], "ok");
    assert_eq!(body["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(body["protocolVersion"], 1);
    assert_eq!(body["adapter"], "auto");
    assert_eq!(body["adapterSelection"], "auto");
    assert!(body["activeAdapter"].is_null());
    let supported = body["supportedAdapters"]
        .as_array()
        .ok_or_else(|| io::Error::other("supportedAdapters is not an array"))?;
    assert_eq!(supported.len(), 4);
    assert_eq!(supported[0]["id"], "f1-24");
    assert_eq!(supported[1]["id"], "f1-25");
    assert_eq!(supported[1]["protocolVersion"], "2025/v3 + 2026/v10");
    assert_eq!(supported[2]["id"], "ets2");
    assert_eq!(supported[3]["id"], "ats");
    assert!(supported.iter().all(|adapter| {
        adapter["packetsRecognized"] == 0 && adapter["lastPacketAgeMs"].is_null()
    }));
    assert_eq!(body["telemetry"]["packetsReceived"], 1);
    assert_eq!(body["telemetry"]["packetsRecognized"], 0);
    assert_eq!(body["telemetry"]["packetErrors"], 1);
    assert!(body["telemetry"]["lastPacketAgeMs"].is_u64());
    assert_eq!(body["telemetry"]["snapshotsPublished"], 0);
    assert_eq!(body["telemetry"]["eventResyncs"], 0);
    assert_eq!(body["connections"]["active"], 0);
    assert_eq!(
        body["connections"]["limit"],
        opensimdash_host::MAX_WEBSOCKET_CONNECTIONS
    );

    let serialized = serde_json::to_string(&body)?;
    for secret_field in ["pairingToken", "deviceSession", "ipAddress", "playerName"] {
        assert!(!serialized.contains(secret_field));
    }

    running.shutdown().await?;
    Ok(())
}

async fn wait_for_packet_diagnostics(
    address: std::net::SocketAddr,
) -> Result<Value, Box<dyn Error>> {
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let body = http_get_json(address, "/api/v1/diagnostics").await?;
            if body["telemetry"]["packetsReceived"] == 1 {
                return Ok::<Value, Box<dyn Error>>(body);
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await?
}

async fn http_get_json(address: std::net::SocketAddr, path: &str) -> Result<Value, Box<dyn Error>> {
    let mut stream = TcpStream::connect(address).await?;
    stream
        .write_all(
            format!("GET {path} HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n")
                .as_bytes(),
        )
        .await?;
    let mut bytes = Vec::new();
    stream.read_to_end(&mut bytes).await?;
    let separator = bytes
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| io::Error::other("HTTP response has no body separator"))?;
    Ok(serde_json::from_slice(&bytes[separator + 4..])?)
}
