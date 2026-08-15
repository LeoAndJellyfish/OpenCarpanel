use std::{error::Error, io};

use opensimdash_adapter_api::{AdapterOutput, GameAdapter};
use opensimdash_adapter_f1::{
    CAR_TELEMETRY_DATA_LEN, CAR_TELEMETRY_PACKET_LEN, DecodeError, F1_24_PACKET_FORMAT,
    F1_24Adapter, F1_25_2026_CAR_COUNT, F1_25_2026_CAR_TELEMETRY_DATA_LEN,
    F1_25_2026_CAR_TELEMETRY_PACKET_LEN, F1_25_2026_PACKET_FORMAT, F1_25_PACKET_FORMAT,
    F1_25Adapter, PACKET_HEADER_LEN, decode_f1_24_player_car_telemetry,
    decode_f1_25_player_car_telemetry,
};
use opensimdash_telemetry_core::{DrsState, Gear, MonotonicTimestamp, Normalized, TelemetryUpdate};

const LEGACY_PLAYER_INDEX: u8 = 7;
const SEASON_PACK_PLAYER_INDEX: u8 = 23;

#[derive(Clone, Copy)]
struct Layout {
    car_data_len: usize,
    packet_len: usize,
    player_index: u8,
}

const LEGACY_LAYOUT: Layout = Layout {
    car_data_len: CAR_TELEMETRY_DATA_LEN,
    packet_len: CAR_TELEMETRY_PACKET_LEN,
    player_index: LEGACY_PLAYER_INDEX,
};

const SEASON_PACK_LAYOUT: Layout = Layout {
    car_data_len: F1_25_2026_CAR_TELEMETRY_DATA_LEN,
    packet_len: F1_25_2026_CAR_TELEMETRY_PACKET_LEN,
    player_index: SEASON_PACK_PLAYER_INDEX,
};

fn car_offset(index: usize, car_data_len: usize) -> usize {
    PACKET_HEADER_LEN + index * car_data_len
}

fn write_u16(packet: &mut [u8], offset: usize, value: u16) {
    packet[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn write_f32(packet: &mut [u8], offset: usize, value: f32) {
    packet[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn synthetic_packet(packet_format: u16, game_year: u8, layout: Layout) -> Vec<u8> {
    let mut packet = Vec::with_capacity(layout.packet_len);
    packet.extend_from_slice(&packet_format.to_le_bytes());
    packet.extend_from_slice(&[game_year, 1, 0, 1, 6]);
    packet.extend_from_slice(&0x1122_3344_5566_7788_u64.to_le_bytes());
    packet.extend_from_slice(&15.25_f32.to_le_bytes());
    packet.extend_from_slice(&88_u32.to_le_bytes());
    packet.extend_from_slice(&100_u32.to_le_bytes());
    packet.extend_from_slice(&[layout.player_index, 255]);
    packet.resize(layout.packet_len, 0);

    let other = car_offset(0, layout.car_data_len);
    write_u16(&mut packet, other, 111);

    let player = car_offset(usize::from(layout.player_index), layout.car_data_len);
    write_u16(&mut packet, player, 324);
    write_f32(&mut packet, player + 2, 0.75);
    write_f32(&mut packet, player + 6, -0.25);
    write_f32(&mut packet, player + 10, 0.25);
    packet[player + 14] = 0;
    packet[player + 15] = 7_i8.to_le_bytes()[0];
    write_u16(&mut packet, player + 16, 11_800);
    packet[player + 18] = 1;
    packet[player + 19] = 84;
    packet
}

fn f1_24_packet() -> Vec<u8> {
    synthetic_packet(F1_24_PACKET_FORMAT, 24, LEGACY_LAYOUT)
}

fn f1_25_packet() -> Vec<u8> {
    synthetic_packet(F1_25_PACKET_FORMAT, 25, LEGACY_LAYOUT)
}

fn f1_25_2026_packet() -> Vec<u8> {
    synthetic_packet(F1_25_2026_PACKET_FORMAT, 26, SEASON_PACK_LAYOUT)
}

fn rejected<T>(result: Result<T, DecodeError>) -> Result<DecodeError, Box<dyn Error>> {
    match result {
        Ok(_) => Err(io::Error::other("packet unexpectedly decoded").into()),
        Err(error) => Ok(error),
    }
}

fn decoded_f1_24(packet: &[u8]) -> Result<TelemetryUpdate, DecodeError> {
    decode_f1_24_player_car_telemetry(packet, MonotonicTimestamp::from_micros(900))
}

fn decoded_f1_25(packet: &[u8]) -> Result<TelemetryUpdate, DecodeError> {
    decode_f1_25_player_car_telemetry(packet, MonotonicTimestamp::from_micros(900))
}

fn assert_verified_vehicle_fields(update: &TelemetryUpdate) -> Result<(), Box<dyn Error>> {
    let speed = update
        .vehicle
        .speed_mps
        .ok_or_else(|| io::Error::other("speed is missing"))?;
    assert!((speed - 90.0).abs() < f32::EPSILON);
    assert_eq!(
        update.vehicle.gear,
        Some(Gear::forward(7).ok_or_else(|| io::Error::other("gear"))?)
    );
    assert_eq!(update.vehicle.rpm, Some(11_800));
    assert_eq!(update.vehicle.rev_lights.map(Normalized::get), Some(0.84));
    assert_eq!(update.vehicle.throttle.map(Normalized::get), Some(0.75));
    assert_eq!(update.vehicle.brake.map(Normalized::get), Some(0.25));
    assert_eq!(update.vehicle.drs, Some(DrsState::Active));
    assert_eq!(update.received_at, MonotonicTimestamp::from_micros(900));
    assert_eq!(update.session_id.as_deref(), Some("1122334455667788"));
    assert_eq!(update.frame_id, Some(100));
    assert!(update.lap.current.is_none());
    assert!(update.extensions.is_empty());
    Ok(())
}

#[test]
fn every_supported_layout_selects_player_and_maps_verified_fields() -> Result<(), Box<dyn Error>> {
    assert_verified_vehicle_fields(&decoded_f1_24(&f1_24_packet())?)?;
    assert_verified_vehicle_fields(&decoded_f1_25(&f1_25_packet())?)?;
    assert_verified_vehicle_fields(&decoded_f1_25(&f1_25_2026_packet())?)?;
    Ok(())
}

#[test]
fn concrete_adapters_have_stable_ids_and_isolated_formats() -> Result<(), Box<dyn Error>> {
    let mut f1_24 = F1_24Adapter::new()?;
    let mut f1_25 = F1_25Adapter::new()?;
    let mut output = AdapterOutput::with_capacity(1, 0);

    f1_24.decode(
        &f1_24_packet(),
        MonotonicTimestamp::from_micros(900),
        &mut output,
    )?;
    assert_eq!(f1_24.descriptor().id.as_str(), "f1-24");
    assert_eq!(output.updates.len(), 1);

    output.clear();
    f1_25.decode(
        &f1_25_packet(),
        MonotonicTimestamp::from_micros(900),
        &mut output,
    )?;
    assert_eq!(f1_25.descriptor().id.as_str(), "f1-25");
    assert_eq!(output.updates.len(), 1);

    output.clear();
    f1_25.decode(
        &f1_25_2026_packet(),
        MonotonicTimestamp::from_micros(900),
        &mut output,
    )?;
    assert_eq!(f1_25.descriptor().id.as_str(), "f1-25");
    assert_eq!(output.updates.len(), 1);

    output.clear();
    assert!(
        f1_24
            .decode(
                &f1_25_packet(),
                MonotonicTimestamp::from_micros(900),
                &mut output,
            )
            .is_err()
    );
    assert!(
        f1_24
            .decode(
                &f1_25_2026_packet(),
                MonotonicTimestamp::from_micros(900),
                &mut output,
            )
            .is_err()
    );
    assert!(
        f1_25
            .decode(
                &f1_24_packet(),
                MonotonicTimestamp::from_micros(900),
                &mut output,
            )
            .is_err()
    );
    Ok(())
}

#[test]
fn season_pack_layout_enforces_24_cars_and_1448_bytes() -> Result<(), Box<dyn Error>> {
    assert_eq!(F1_25_2026_CAR_COUNT, 24);

    let packet = f1_25_2026_packet();
    assert_eq!(packet.len(), 1_448);
    assert_eq!(F1_25_2026_CAR_TELEMETRY_DATA_LEN, 59);
    assert_eq!(F1_25_2026_CAR_TELEMETRY_PACKET_LEN, 1_448);

    assert_eq!(
        rejected(decoded_f1_25(&packet[..CAR_TELEMETRY_PACKET_LEN]))?,
        DecodeError::InvalidPacketLength {
            packet_id: 6,
            expected: F1_25_2026_CAR_TELEMETRY_PACKET_LEN,
            actual: CAR_TELEMETRY_PACKET_LEN,
        }
    );

    let mut invalid_index = packet;
    invalid_index[27] = 24;
    assert_eq!(
        rejected(decoded_f1_25(&invalid_index))?,
        DecodeError::InvalidPlayerIndex {
            index: 24,
            car_count: F1_25_2026_CAR_COUNT,
        }
    );
    Ok(())
}

#[test]
fn malformed_f1_25_packets_are_rejected_without_clamping() -> Result<(), Box<dyn Error>> {
    let packet = f1_25_packet();
    assert_eq!(
        rejected(decoded_f1_25(&packet[..packet.len() - 1]))?,
        DecodeError::InvalidPacketLength {
            packet_id: 6,
            expected: CAR_TELEMETRY_PACKET_LEN,
            actual: CAR_TELEMETRY_PACKET_LEN - 1,
        }
    );

    let mut packet = f1_25_packet();
    packet[27] = 22;
    assert_eq!(
        rejected(decoded_f1_25(&packet))?,
        DecodeError::InvalidPlayerIndex {
            index: 22,
            car_count: 22,
        }
    );

    let player = car_offset(usize::from(LEGACY_PLAYER_INDEX), CAR_TELEMETRY_DATA_LEN);
    for (expected_field, offset, value) in [
        ("throttle", player + 2, f32::NAN),
        ("throttle", player + 2, -0.01),
        ("throttle", player + 2, 1.01),
        ("brake", player + 10, f32::NAN),
        ("brake", player + 10, -0.01),
        ("brake", player + 10, 1.01),
    ] {
        let mut packet = f1_25_packet();
        write_f32(&mut packet, offset, value);
        let error = rejected(decoded_f1_25(&packet))?;
        assert!(matches!(
            error,
            DecodeError::InvalidNormalizedValue { field, .. } if field == expected_field
        ));
    }

    Ok(())
}

#[test]
fn invalid_version_drs_and_unknown_gear_remain_explicit() -> Result<(), Box<dyn Error>> {
    let player = car_offset(usize::from(LEGACY_PLAYER_INDEX), CAR_TELEMETRY_DATA_LEN);
    let mut packet = f1_25_packet();
    packet[player + 15] = 9_i8.to_le_bytes()[0];
    assert_eq!(decoded_f1_25(&packet)?.vehicle.gear, Some(Gear::Unknown));

    packet[player + 18] = 2;
    assert_eq!(
        rejected(decoded_f1_25(&packet))?,
        DecodeError::InvalidEnumValue {
            field: "drs",
            actual: 2,
        }
    );

    let mut packet = f1_25_packet();
    packet[5] = 2;
    assert_eq!(
        rejected(decoded_f1_25(&packet))?,
        DecodeError::UnsupportedPacketVersion {
            packet_id: 6,
            expected: 1,
            actual: 2,
        }
    );
    Ok(())
}
