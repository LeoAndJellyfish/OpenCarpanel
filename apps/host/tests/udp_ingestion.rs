use std::{error::Error, time::Duration};

use opensimdash_adapter_f1::{CAR_TELEMETRY_PACKET_LEN, PACKET_HEADER_LEN};
use opensimdash_host::spawn_host;
use opensimdash_telemetry_core::Gear;
use tokio::net::{TcpListener, UdpSocket};

fn synthetic_player_packet() -> Vec<u8> {
    let mut packet = Vec::with_capacity(CAR_TELEMETRY_PACKET_LEN);
    packet.extend_from_slice(&2024_u16.to_le_bytes());
    packet.extend_from_slice(&[24, 1, 0, 1, 6]);
    packet.extend_from_slice(&0x0102_0304_0506_0708_u64.to_le_bytes());
    packet.extend_from_slice(&1.0_f32.to_le_bytes());
    packet.extend_from_slice(&10_u32.to_le_bytes());
    packet.extend_from_slice(&12_u32.to_le_bytes());
    packet.extend_from_slice(&[0, 255]);
    packet.resize(CAR_TELEMETRY_PACKET_LEN, 0);

    let player = PACKET_HEADER_LEN;
    packet[player..player + 2].copy_from_slice(&360_u16.to_le_bytes());
    packet[player + 2..player + 6].copy_from_slice(&0.8_f32.to_le_bytes());
    packet[player + 10..player + 14].copy_from_slice(&0.1_f32.to_le_bytes());
    packet[player + 15] = 8_i8.to_le_bytes()[0];
    packet[player + 16..player + 18].copy_from_slice(&12_300_u16.to_le_bytes());
    packet[player + 18] = 1;
    packet
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn udp_datagram_updates_the_latest_snapshot_without_a_queue() -> Result<(), Box<dyn Error>> {
    let http_listener = TcpListener::bind("127.0.0.1:0").await?;
    let udp_socket = UdpSocket::bind("127.0.0.1:0").await?;
    let running = spawn_host(http_listener, udp_socket)?;
    let mut snapshots = running.state().subscribe_snapshots();
    let sender = UdpSocket::bind("127.0.0.1:0").await?;

    sender
        .send_to(&synthetic_player_packet(), running.udp_address())
        .await?;
    tokio::time::timeout(Duration::from_secs(2), snapshots.changed()).await??;

    let snapshot = snapshots.borrow().clone();
    assert_eq!(snapshot.meta.game_id.as_deref(), Some("f1-24"));
    assert_eq!(
        snapshot.meta.session_id.as_deref(),
        Some("0102030405060708")
    );
    assert_eq!(snapshot.vehicle.speed_mps, Some(100.0));
    assert_eq!(snapshot.vehicle.rpm, Some(12_300));
    assert_eq!(
        snapshot.vehicle.gear,
        Gear::forward(8).unwrap_or(Gear::Unknown)
    );
    assert_eq!(running.state().metrics().packets_received, 1);
    assert_eq!(running.state().metrics().packet_errors, 0);

    drop(snapshots);
    running.shutdown().await?;
    Ok(())
}
