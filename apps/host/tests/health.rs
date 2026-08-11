use std::{error::Error, io};

use opencarpanel_host::spawn_host;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream, UdpSocket},
};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn health_endpoint_reports_protocol_and_adapter_then_shuts_down() -> Result<(), Box<dyn Error>>
{
    let http_listener = TcpListener::bind("127.0.0.1:0").await?;
    let udp_socket = UdpSocket::bind("127.0.0.1:0").await?;
    let running = spawn_host(http_listener, udp_socket).await?;
    let http_address = running.http_address();

    let response = http_get(http_address, "/api/v1/health").await?;
    let (head, body) = response
        .split_once("\r\n\r\n")
        .ok_or_else(|| io::Error::other("HTTP response has no body separator"))?;
    assert!(head.starts_with("HTTP/1.1 200"));
    let body: serde_json::Value = serde_json::from_str(body)?;
    assert_eq!(
        body,
        serde_json::json!({
            "status": "ok",
            "protocolVersion": 1,
            "adapter": "f1-24"
        })
    );

    running.shutdown().await?;
    let rebound = TcpListener::bind(http_address).await?;
    drop(rebound);
    Ok(())
}

async fn http_get(address: std::net::SocketAddr, path: &str) -> Result<String, Box<dyn Error>> {
    let mut stream = TcpStream::connect(address).await?;
    stream
        .write_all(
            format!("GET {path} HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n")
                .as_bytes(),
        )
        .await?;
    let mut bytes = Vec::new();
    stream.read_to_end(&mut bytes).await?;
    Ok(String::from_utf8(bytes)?)
}
