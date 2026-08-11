use std::{error::Error, io};

use opencarpanel_host::spawn_host;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream, UdpSocket},
};

#[derive(Debug)]
struct HttpResponse {
    status: u16,
    head: String,
    body: String,
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn host_serves_the_spa_with_strict_headers_and_never_masks_unknown_api_routes()
-> Result<(), Box<dyn Error>> {
    let running = spawn_host(
        TcpListener::bind("127.0.0.1:0").await?,
        UdpSocket::bind("127.0.0.1:0").await?,
    )?;
    let address = running.http_address();

    for path in ["/", "/edit"] {
        let response = get(address, path).await?;
        assert_eq!(response.status, 200);
        assert!(
            response
                .head
                .contains("content-type: text/html; charset=utf-8")
        );
        assert!(response.head.contains("cache-control: no-store"));
        assert!(response.head.contains("x-content-type-options: nosniff"));
        assert!(response.head.contains("x-frame-options: deny"));
        assert!(response.head.contains("referrer-policy: no-referrer"));
        assert!(response.head.contains("content-security-policy:"));
        assert!(!response.head.contains("unsafe-inline"));
        assert!(response.body.contains("OpenCarpanel"));
        assert!(!response.body.contains("https://"));
    }

    let index = get(address, "/").await?;
    if let Some(asset_path) = first_asset_path(&index.body) {
        let asset = get(address, asset_path).await?;
        assert_eq!(asset.status, 200);
        assert!(
            asset
                .head
                .contains("cache-control: public, max-age=31536000, immutable")
        );
        assert!(asset.head.contains("x-content-type-options: nosniff"));
    }

    assert_eq!(get(address, "/assets/missing.js").await?.status, 404);
    let missing_api = get(address, "/api/v1/unknown").await?;
    assert_eq!(missing_api.status, 404);
    assert!(!missing_api.body.contains("<html"));

    running.shutdown().await?;
    Ok(())
}

fn first_asset_path(html: &str) -> Option<&str> {
    let start = html.find("/assets/")?;
    let rest = &html[start..];
    let end = rest.find(['\"', '\'', '<', '>'])?;
    Some(&rest[..end])
}

async fn get(address: std::net::SocketAddr, path: &str) -> Result<HttpResponse, Box<dyn Error>> {
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
    let head = String::from_utf8(bytes[..separator].to_vec())?;
    let status = head
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| io::Error::other("HTTP response has no status"))?
        .parse()?;
    Ok(HttpResponse {
        status,
        head: head.to_ascii_lowercase(),
        body: String::from_utf8(bytes[separator + 4..].to_vec())?,
    })
}
