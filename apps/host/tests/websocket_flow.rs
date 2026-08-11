use std::{error::Error, io, time::Duration};

use futures_util::{SinkExt, StreamExt};
use opencarpanel_host::{EVENT_BUFFER_CAPACITY, RunningHost, spawn_host};
use opencarpanel_telemetry_core::{MonotonicTimestamp, TelemetryEvent, TelemetrySnapshot};
use serde_json::{Value, json};
use tokio::net::{TcpListener, UdpSocket};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async,
    tungstenite::{Message, protocol::CloseFrame},
};

type ClientSocket = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

async fn host() -> Result<RunningHost, Box<dyn Error>> {
    Ok(spawn_host(
        TcpListener::bind("127.0.0.1:0").await?,
        UdpSocket::bind("127.0.0.1:0").await?,
    )
    .await?)
}

async fn connect(host: &RunningHost) -> Result<ClientSocket, Box<dyn Error>> {
    let (socket, _response) =
        connect_async(format!("ws://{}/api/v1/ws", host.http_address())).await?;
    Ok(socket)
}

async fn send_json(socket: &mut ClientSocket, value: Value) -> Result<(), Box<dyn Error>> {
    socket.send(Message::Text(value.to_string().into())).await?;
    Ok(())
}

async fn next_json(socket: &mut ClientSocket) -> Result<Value, Box<dyn Error>> {
    loop {
        let message = tokio::time::timeout(Duration::from_secs(2), socket.next())
            .await?
            .ok_or_else(|| io::Error::other("WebSocket closed before JSON message"))??;
        if let Message::Text(text) = message {
            return Ok(serde_json::from_str(text.as_ref())?);
        }
    }
}

async fn next_type(socket: &mut ClientSocket, expected: &str) -> Result<Value, Box<dyn Error>> {
    loop {
        let value = next_json(socket).await?;
        if value["type"] == expected {
            return Ok(value);
        }
    }
}

async fn pair(host: &RunningHost, token: &str) -> Result<(ClientSocket, String), Box<dyn Error>> {
    let mut socket = connect(host).await?;
    send_json(
        &mut socket,
        json!({
            "v": 1,
            "type": "hello",
            "pairingToken": token,
            "lastEventSeq": 0,
            "snapshotHz": 60
        }),
    )
    .await?;
    let hello = next_type(&mut socket, "hello").await?;
    let session = hello["deviceSession"]
        .as_str()
        .ok_or_else(|| io::Error::other("server did not issue a device session"))?
        .to_owned();
    Ok((socket, session))
}

async fn resume(
    host: &RunningHost,
    session: &str,
    last_event_seq: u64,
) -> Result<ClientSocket, Box<dyn Error>> {
    let mut socket = connect(host).await?;
    send_json(
        &mut socket,
        json!({
            "v": 1,
            "type": "hello",
            "deviceSession": session,
            "lastEventSeq": last_event_seq,
            "snapshotHz": 60
        }),
    )
    .await?;
    let _hello = next_type(&mut socket, "hello").await?;
    Ok(socket)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn pairing_token_is_one_time_and_expired_tokens_are_typed_errors()
-> Result<(), Box<dyn Error>> {
    let host = host().await?;
    let token = host.issue_pairing_token(Duration::from_secs(30)).await?;
    let (mut first, _session) = pair(&host, &token).await?;
    first.close(None).await?;

    let mut reused = connect(&host).await?;
    send_json(
        &mut reused,
        json!({"v":1,"type":"hello","pairingToken":token,"snapshotHz":60}),
    )
    .await?;
    let error = next_type(&mut reused, "error").await?;
    assert_eq!(error["code"], "invalid_pairing_token");

    let expired = host.issue_pairing_token(Duration::ZERO).await?;
    let mut expired_socket = connect(&host).await?;
    send_json(
        &mut expired_socket,
        json!({"v":1,"type":"hello","pairingToken":expired,"snapshotHz":60}),
    )
    .await?;
    let error = next_type(&mut expired_socket, "error").await?;
    assert_eq!(error["code"], "pairing_token_expired");

    drop(reused);
    drop(expired_socket);
    host.shutdown().await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn snapshot_lane_collapses_bursts_to_the_newest_sequence() -> Result<(), Box<dyn Error>> {
    let host = host().await?;
    let token = host.issue_pairing_token(Duration::from_secs(30)).await?;
    let (mut socket, _session) = pair(&host, &token).await?;
    let _initial = next_type(&mut socket, "snapshot").await?;

    for sequence in 1..=100 {
        let mut snapshot = TelemetrySnapshot::default();
        snapshot.meta.sequence = sequence;
        host.state().replace_snapshot(snapshot);
    }

    let newest = next_type(&mut socket, "snapshot").await?;
    assert_eq!(newest["seq"], 100);
    socket.close(None).await?;
    host.shutdown().await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn events_replay_in_order_and_ring_overflow_requires_resync() -> Result<(), Box<dyn Error>> {
    let host = host().await?;
    let token = host.issue_pairing_token(Duration::from_secs(30)).await?;
    let (mut initial, session) = pair(&host, &token).await?;
    initial.close(None).await?;

    for number in 1..=3 {
        host.state()
            .publish_event(TelemetryEvent {
                name: format!("test.event.{number}"),
                occurred_at: MonotonicTimestamp::from_micros(number),
                data: json!({"number": number}),
            })
            .await;
    }
    let mut replayed = resume(&host, &session, 1).await?;
    let second = next_type(&mut replayed, "event").await?;
    let third = next_type(&mut replayed, "event").await?;
    assert_eq!(second["seq"], 2);
    assert_eq!(third["seq"], 3);
    replayed.close(None).await?;

    for number in 4..=(EVENT_BUFFER_CAPACITY as u64 + 4) {
        host.state()
            .publish_event(TelemetryEvent {
                name: "test.overflow".into(),
                occurred_at: MonotonicTimestamp::from_micros(number),
                data: json!({"number": number}),
            })
            .await;
    }
    let mut behind = resume(&host, &session, 0).await?;
    let resync = next_type(&mut behind, "resync_required").await?;
    assert!(
        resync["oldestAvailableEventSeq"]
            .as_u64()
            .is_some_and(|seq| seq > 1)
    );
    behind.close(None).await?;
    host.shutdown().await?;
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn unsupported_protocol_and_oversized_messages_close_with_typed_reasons()
-> Result<(), Box<dyn Error>> {
    let host = host().await?;
    let mut unsupported = connect(&host).await?;
    send_json(
        &mut unsupported,
        json!({"v":99,"type":"hello","pairingToken":"unused","snapshotHz":60}),
    )
    .await?;
    let error = next_type(&mut unsupported, "error").await?;
    assert_eq!(error["code"], "unsupported_version");
    let close = next_close(&mut unsupported).await?;
    assert_eq!(u16::from(close.code), 1002);
    assert_eq!(close.reason, "unsupported_version");

    let mut oversized = connect(&host).await?;
    oversized
        .send(Message::Text("x".repeat(20_000).into()))
        .await?;
    let close = next_close(&mut oversized).await?;
    assert_eq!(u16::from(close.code), 1009);
    host.shutdown().await?;
    Ok(())
}

async fn next_close(socket: &mut ClientSocket) -> Result<CloseFrame, Box<dyn Error>> {
    loop {
        let message = tokio::time::timeout(Duration::from_secs(2), socket.next())
            .await?
            .ok_or_else(|| io::Error::other("WebSocket ended without close frame"))??;
        if let Message::Close(Some(frame)) = message {
            return Ok(frame);
        }
    }
}
