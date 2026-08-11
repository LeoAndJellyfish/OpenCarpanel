use std::{error::Error, io};

use opencarpanel_adapter_api::{AdapterOutput, GameAdapter};
use opencarpanel_adapter_f1_24::{
    CAR_TELEMETRY_DATA_LEN, CAR_TELEMETRY_PACKET_LEN, DecodeError, F1_24Adapter, PACKET_HEADER_LEN,
    decode_player_car_telemetry,
};
use opencarpanel_telemetry_core::{
    DrsState, Gear, MonotonicTimestamp, Normalized, TelemetryUpdate,
};

const PLAYER_INDEX: u8 = 7;

fn car_offset(index: usize) -> usize {
    PACKET_HEADER_LEN + index * CAR_TELEMETRY_DATA_LEN
}

fn write_u16(packet: &mut [u8], offset: usize, value: u16) {
    packet[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn write_f32(packet: &mut [u8], offset: usize, value: f32) {
    packet[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn synthetic_packet() -> Vec<u8> {
    let mut packet = Vec::with_capacity(CAR_TELEMETRY_PACKET_LEN);
    packet.extend_from_slice(&2024_u16.to_le_bytes());
    packet.extend_from_slice(&[24, 1, 0, 1, 6]);
    packet.extend_from_slice(&0x1122_3344_5566_7788_u64.to_le_bytes());
    packet.extend_from_slice(&15.25_f32.to_le_bytes());
    packet.extend_from_slice(&88_u32.to_le_bytes());
    packet.extend_from_slice(&100_u32.to_le_bytes());
    packet.extend_from_slice(&[PLAYER_INDEX, 255]);
    packet.resize(CAR_TELEMETRY_PACKET_LEN, 0);

    let other = car_offset(0);
    write_u16(&mut packet, other, 111);

    let player = car_offset(usize::from(PLAYER_INDEX));
    write_u16(&mut packet, player, 324);
    write_f32(&mut packet, player + 2, 0.75);
    write_f32(&mut packet, player + 6, -0.25);
    write_f32(&mut packet, player + 10, 0.25);
    packet[player + 14] = 0;
    packet[player + 15] = 7_i8.to_le_bytes()[0];
    write_u16(&mut packet, player + 16, 11_800);
    packet[player + 18] = 1;
    packet
}

fn rejected<T>(result: Result<T, DecodeError>) -> Result<DecodeError, Box<dyn Error>> {
    match result {
        Ok(_) => Err(io::Error::other("packet unexpectedly decoded").into()),
        Err(error) => Ok(error),
    }
}

fn decoded(packet: &[u8]) -> Result<TelemetryUpdate, DecodeError> {
    decode_player_car_telemetry(packet, MonotonicTimestamp::from_micros(900))
}

#[test]
fn selects_player_and_maps_verified_vehicle_fields() -> Result<(), Box<dyn Error>> {
    let update = decoded(&synthetic_packet())?;

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
fn game_adapter_appends_one_update_to_reusable_output() -> Result<(), Box<dyn Error>> {
    let mut adapter = F1_24Adapter::new()?;
    let mut output = AdapterOutput::with_capacity(1, 0);

    adapter.decode(
        &synthetic_packet(),
        MonotonicTimestamp::from_micros(900),
        &mut output,
    )?;

    assert_eq!(adapter.descriptor().id.as_str(), "f1-24");
    assert_eq!(output.updates.len(), 1);
    assert!(output.events.is_empty());
    Ok(())
}

#[test]
fn truncated_car_array_is_rejected() -> Result<(), Box<dyn Error>> {
    let packet = synthetic_packet();
    let error = rejected(decoded(&packet[..packet.len() - 1]))?;

    assert_eq!(
        error,
        DecodeError::InvalidPacketLength {
            packet_id: 6,
            expected: CAR_TELEMETRY_PACKET_LEN,
            actual: CAR_TELEMETRY_PACKET_LEN - 1,
        }
    );
    Ok(())
}

#[test]
fn invalid_player_index_is_rejected() -> Result<(), Box<dyn Error>> {
    let mut packet = synthetic_packet();
    packet[27] = 22;
    let error = rejected(decoded(&packet))?;

    assert_eq!(
        error,
        DecodeError::InvalidPlayerIndex {
            index: 22,
            car_count: 22,
        }
    );
    Ok(())
}

#[test]
fn corrupt_normalized_inputs_are_rejected_without_clamping() -> Result<(), Box<dyn Error>> {
    let player = car_offset(usize::from(PLAYER_INDEX));
    let cases = [
        ("throttle", player + 2, f32::NAN),
        ("throttle", player + 2, -0.01),
        ("throttle", player + 2, 1.01),
        ("brake", player + 10, f32::NAN),
        ("brake", player + 10, -0.01),
        ("brake", player + 10, 1.01),
    ];

    for (expected_field, offset, value) in cases {
        let mut packet = synthetic_packet();
        write_f32(&mut packet, offset, value);
        let error = rejected(decoded(&packet))?;
        assert!(matches!(
            error,
            DecodeError::InvalidNormalizedValue { field, .. } if field == expected_field
        ));
    }

    Ok(())
}

#[test]
fn unknown_gear_is_explicit_and_invalid_drs_is_rejected() -> Result<(), Box<dyn Error>> {
    let player = car_offset(usize::from(PLAYER_INDEX));
    let mut packet = synthetic_packet();
    packet[player + 15] = 9_i8.to_le_bytes()[0];
    assert_eq!(decoded(&packet)?.vehicle.gear, Some(Gear::Unknown));

    packet[player + 18] = 2;
    let error = rejected(decoded(&packet))?;
    assert_eq!(
        error,
        DecodeError::InvalidEnumValue {
            field: "drs",
            actual: 2,
        }
    );
    Ok(())
}

#[test]
fn wrong_packet_version_is_rejected() -> Result<(), Box<dyn Error>> {
    let mut packet = synthetic_packet();
    packet[5] = 2;
    let error = rejected(decoded(&packet))?;

    assert_eq!(
        error,
        DecodeError::UnsupportedPacketVersion {
            packet_id: 6,
            expected: 1,
            actual: 2,
        }
    );
    Ok(())
}
