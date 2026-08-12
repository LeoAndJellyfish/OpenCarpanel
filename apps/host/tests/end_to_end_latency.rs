use std::{
    error::Error,
    io,
    time::{Duration, Instant},
};

use futures_util::{SinkExt, StreamExt};
use opencarpanel_adapter_f1::{CAR_TELEMETRY_PACKET_LEN, PACKET_HEADER_LEN};
use opencarpanel_host::spawn_host;
use serde_json::{Value, json};
use tokio::net::{TcpListener, UdpSocket};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async, tungstenite::Message};

type ClientSocket = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;
const SAMPLE_COUNT: u32 = 120;
const LATENCY_BUDGET: Duration = Duration::from_millis(100);

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn synthetic_udp_to_websocket_p95_stays_below_100_ms() -> Result<(), Box<dyn Error>> {
    let running = spawn_host(
        TcpListener::bind("127.0.0.1:0").await?,
        UdpSocket::bind("127.0.0.1:0").await?,
    )?;
    let token = running.issue_pairing_token(Duration::from_secs(30)).await?;
    let (mut socket, _response) =
        connect_async(format!("ws://{}/api/v1/ws", running.http_address())).await?;
    send_json(
        &mut socket,
        json!({"v":1,"type":"hello","pairingToken":token,"snapshotHz":60}),
    )
    .await?;
    let _hello = next_type(&mut socket, "hello").await?;
    let _initial = next_type(&mut socket, "snapshot").await?;

    let sender = UdpSocket::bind("127.0.0.1:0").await?;
    let mut latencies = Vec::with_capacity(SAMPLE_COUNT as usize);
    for frame in 1..=SAMPLE_COUNT {
        let speed_kph = u16::try_from(200 + frame)?;
        let packet = synthetic_player_packet(frame, speed_kph);
        let started = Instant::now();
        sender.send_to(&packet, running.udp_address()).await?;
        let snapshot = next_snapshot_at_or_after(&mut socket, u64::from(frame)).await?;
        latencies.push(started.elapsed());

        assert_eq!(snapshot["seq"], frame);
        let speed_mps = snapshot["data"]["vehicle"]["speedMps"]
            .as_f64()
            .ok_or_else(|| io::Error::other("snapshot has no numeric speed"))?;
        assert!((speed_mps - f64::from(speed_kph) / 3.6).abs() < 0.001);
    }

    latencies.sort_unstable();
    let p50 = percentile(&latencies, 50);
    let p95 = percentile(&latencies, 95);
    let p99 = percentile(&latencies, 99);
    println!("synthetic UDP-to-WebSocket latency: p50={p50:?} p95={p95:?} p99={p99:?}");
    assert!(
        p95 < LATENCY_BUDGET,
        "synthetic UDP-to-WebSocket p95 {p95:?} exceeded {LATENCY_BUDGET:?}"
    );

    socket.close(None).await?;
    running.shutdown().await?;
    Ok(())
}

fn percentile(sorted: &[Duration], percent: usize) -> Duration {
    let index = (sorted.len() * percent).div_ceil(100).saturating_sub(1);
    sorted[index]
}

fn synthetic_player_packet(frame: u32, speed_kph: u16) -> Vec<u8> {
    let mut packet = Vec::with_capacity(CAR_TELEMETRY_PACKET_LEN);
    packet.extend_from_slice(&2024_u16.to_le_bytes());
    packet.extend_from_slice(&[24, 1, 0, 1, 6]);
    packet.extend_from_slice(&0x0102_0304_0506_0708_u64.to_le_bytes());
    packet.extend_from_slice(&1.0_f32.to_le_bytes());
    packet.extend_from_slice(&frame.to_le_bytes());
    packet.extend_from_slice(&frame.to_le_bytes());
    packet.extend_from_slice(&[0, 255]);
    packet.resize(CAR_TELEMETRY_PACKET_LEN, 0);

    let player = PACKET_HEADER_LEN;
    packet[player..player + 2].copy_from_slice(&speed_kph.to_le_bytes());
    packet[player + 2..player + 6].copy_from_slice(&0.8_f32.to_le_bytes());
    packet[player + 10..player + 14].copy_from_slice(&0.1_f32.to_le_bytes());
    packet[player + 15] = 7_i8.to_le_bytes()[0];
    packet[player + 16..player + 18].copy_from_slice(&11_500_u16.to_le_bytes());
    packet[player + 18] = 1;
    packet[player + 19] = 80;
    packet
}

async fn send_json(socket: &mut ClientSocket, value: Value) -> Result<(), Box<dyn Error>> {
    socket.send(Message::Text(value.to_string().into())).await?;
    Ok(())
}

async fn next_snapshot_at_or_after(
    socket: &mut ClientSocket,
    sequence: u64,
) -> Result<Value, Box<dyn Error>> {
    loop {
        let snapshot = next_type(socket, "snapshot").await?;
        if snapshot["seq"]
            .as_u64()
            .is_some_and(|value| value >= sequence)
        {
            return Ok(snapshot);
        }
    }
}

async fn next_type(socket: &mut ClientSocket, expected: &str) -> Result<Value, Box<dyn Error>> {
    loop {
        let message = tokio::time::timeout(Duration::from_secs(2), socket.next())
            .await?
            .ok_or_else(|| io::Error::other("WebSocket closed before expected message"))??;
        if let Message::Text(text) = message {
            let value: Value = serde_json::from_str(text.as_ref())?;
            if value["type"] == expected {
                return Ok(value);
            }
        }
    }
}
