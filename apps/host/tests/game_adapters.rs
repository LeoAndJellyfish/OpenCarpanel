use std::{error::Error, time::Duration};

use opencarpanel_adapter_f1::{
    CAR_TELEMETRY_DATA_LEN, CAR_TELEMETRY_PACKET_LEN, F1_25_2026_CAR_TELEMETRY_DATA_LEN,
    F1_25_2026_CAR_TELEMETRY_PACKET_LEN, F1_25_2026_PACKET_FORMAT, PACKET_HEADER_LEN,
};
use opencarpanel_adapter_scs::{
    ATS_GAME_ID, BRIDGE_MAGIC, BRIDGE_PROTOCOL_V1, BRIDGE_V1_PACKET_LEN, ETS2_GAME_ID,
};
use opencarpanel_host::{AdapterSelection, spawn_host, spawn_host_with_adapter_selection};
use tokio::net::{TcpListener, UdpSocket};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn auto_selection_recognizes_every_built_in_game() -> Result<(), Box<dyn Error>> {
    let cases = [
        (f1_packet(2024, 24), "f1-24"),
        (f1_packet(2025, 25), "f1-25"),
        (f1_packet(F1_25_2026_PACKET_FORMAT, 26), "f1-25"),
        (scs_packet(ETS2_GAME_ID), "ets2"),
        (scs_packet(ATS_GAME_ID), "ats"),
    ];

    for (packet, expected_adapter) in cases {
        let running = spawn_host(
            TcpListener::bind("127.0.0.1:0").await?,
            UdpSocket::bind("127.0.0.1:0").await?,
        )?;
        let mut snapshots = running.state().subscribe_snapshots();
        let sender = UdpSocket::bind("127.0.0.1:0").await?;

        sender.send_to(&packet, running.udp_address()).await?;
        tokio::time::timeout(Duration::from_secs(2), snapshots.changed()).await??;

        assert_eq!(
            snapshots.borrow().meta.game_id.as_deref(),
            Some(expected_adapter)
        );
        assert_eq!(running.state().active_adapter_id(), Some(expected_adapter));
        assert_eq!(running.state().metrics().packets_received, 1);
        assert_eq!(running.state().metrics().packets_recognized, 1);
        assert_eq!(running.state().metrics().packet_errors, 0);

        drop(snapshots);
        running.shutdown().await?;
    }
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn fixed_selection_rejects_another_game_before_accepting_its_own()
-> Result<(), Box<dyn Error>> {
    let running = spawn_host_with_adapter_selection(
        TcpListener::bind("127.0.0.1:0").await?,
        UdpSocket::bind("127.0.0.1:0").await?,
        AdapterSelection::F1_25,
    )?;
    let mut snapshots = running.state().subscribe_snapshots();
    let sender = UdpSocket::bind("127.0.0.1:0").await?;

    sender
        .send_to(&f1_packet(2024, 24), running.udp_address())
        .await?;
    wait_for_packet_error(&running).await?;
    assert_eq!(running.state().active_adapter_id(), None);
    assert_eq!(running.state().metrics().packets_recognized, 0);
    assert_eq!(running.state().metrics().packet_errors, 1);

    sender
        .send_to(&f1_packet(2025, 25), running.udp_address())
        .await?;
    tokio::time::timeout(Duration::from_secs(2), snapshots.changed()).await??;
    assert_eq!(snapshots.borrow().meta.game_id.as_deref(), Some("f1-25"));
    assert_eq!(running.state().active_adapter_id(), Some("f1-25"));
    assert_eq!(running.state().metrics().packets_recognized, 1);
    assert_eq!(running.state().metrics().packet_errors, 1);

    sender
        .send_to(
            &f1_packet_with_frame(F1_25_2026_PACKET_FORMAT, 26, 13),
            running.udp_address(),
        )
        .await?;
    tokio::time::timeout(Duration::from_secs(2), snapshots.changed()).await??;
    assert_eq!(snapshots.borrow().meta.game_id.as_deref(), Some("f1-25"));
    assert_eq!(running.state().active_adapter_id(), Some("f1-25"));
    assert_eq!(running.state().metrics().packets_recognized, 2);
    assert_eq!(running.state().metrics().packet_errors, 1);

    drop(snapshots);
    running.shutdown().await?;
    Ok(())
}

async fn wait_for_packet_error(
    running: &opencarpanel_host::RunningHost,
) -> Result<(), tokio::time::error::Elapsed> {
    tokio::time::timeout(Duration::from_secs(2), async {
        while running.state().metrics().packet_errors == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
}

fn f1_packet(packet_format: u16, game_year: u8) -> Vec<u8> {
    f1_packet_with_frame(packet_format, game_year, 12)
}

fn f1_packet_with_frame(packet_format: u16, game_year: u8, overall_frame: u32) -> Vec<u8> {
    let (car_data_len, packet_len, player_index) = if packet_format == F1_25_2026_PACKET_FORMAT {
        (
            F1_25_2026_CAR_TELEMETRY_DATA_LEN,
            F1_25_2026_CAR_TELEMETRY_PACKET_LEN,
            23_u8,
        )
    } else {
        (CAR_TELEMETRY_DATA_LEN, CAR_TELEMETRY_PACKET_LEN, 0_u8)
    };
    let mut packet = Vec::with_capacity(packet_len);
    packet.extend_from_slice(&packet_format.to_le_bytes());
    packet.extend_from_slice(&[game_year, 1, 0, 1, 6]);
    packet.extend_from_slice(&0x0102_0304_0506_0708_u64.to_le_bytes());
    packet.extend_from_slice(&1.0_f32.to_le_bytes());
    packet.extend_from_slice(&10_u32.to_le_bytes());
    packet.extend_from_slice(&overall_frame.to_le_bytes());
    packet.extend_from_slice(&[player_index, 255]);
    packet.resize(packet_len, 0);

    let player = PACKET_HEADER_LEN + usize::from(player_index) * car_data_len;
    packet[player..player + 2].copy_from_slice(&360_u16.to_le_bytes());
    packet[player + 2..player + 6].copy_from_slice(&0.8_f32.to_le_bytes());
    packet[player + 10..player + 14].copy_from_slice(&0.1_f32.to_le_bytes());
    packet[player + 15] = 8_i8.to_le_bytes()[0];
    packet[player + 16..player + 18].copy_from_slice(&12_300_u16.to_le_bytes());
    packet[player + 18] = 1;
    packet
}

fn scs_packet(game: u8) -> Vec<u8> {
    let mut packet = Vec::with_capacity(BRIDGE_V1_PACKET_LEN);
    packet.extend_from_slice(&BRIDGE_MAGIC);
    packet.extend_from_slice(&[BRIDGE_PROTOCOL_V1, game, 0, 0]);
    packet.extend_from_slice(&0x1112_1314_1516_1718_u64.to_le_bytes());
    packet.extend_from_slice(&42_u32.to_le_bytes());
    packet.extend_from_slice(&(-20.0_f32).to_le_bytes());
    packet.extend_from_slice(&1_300.0_f32.to_le_bytes());
    packet.extend_from_slice(&2_500.0_f32.to_le_bytes());
    packet.extend_from_slice(&6_i32.to_le_bytes());
    packet.extend_from_slice(&0.75_f32.to_le_bytes());
    packet.extend_from_slice(&0.1_f32.to_le_bytes());
    assert_eq!(packet.len(), BRIDGE_V1_PACKET_LEN);
    packet
}
