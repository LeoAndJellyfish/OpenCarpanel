use crate::{Cursor, DecodeError, PacketHeader};

use super::{CAR_TELEMETRY_PACKET_ID, CAR_TELEMETRY_PACKET_VERSION, CarTelemetryLayout};

const MAPPED_PLAYER_FIELDS_LEN: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct CarTelemetrySample {
    pub(crate) speed_kph: u16,
    pub(crate) throttle: f32,
    pub(crate) brake: f32,
    pub(crate) gear: i8,
    pub(crate) engine_rpm: u16,
    pub(crate) drs: u8,
    pub(crate) rev_lights_percent: u8,
}

pub(crate) fn decode_player_sample(
    header: &PacketHeader,
    payload: &[u8],
    datagram_len: usize,
    layout: CarTelemetryLayout,
) -> Result<CarTelemetrySample, DecodeError> {
    validate_header_and_length(header, datagram_len, layout)?;

    let player_index = usize::from(header.player_car_index);
    if player_index >= layout.car_count {
        return Err(DecodeError::InvalidPlayerIndex {
            index: header.player_car_index,
            car_count: layout.car_count,
        });
    }

    let mut cursor = Cursor::new(payload);
    cursor.skip(player_index * layout.car_telemetry_data_len)?;
    let speed_kph = cursor.read_u16_le()?;
    let throttle = cursor.read_f32_le()?;
    let _steer = cursor.read_f32_le()?;
    let brake = cursor.read_f32_le()?;
    let _clutch = cursor.read_u8()?;
    let gear = cursor.read_i8()?;
    let engine_rpm = cursor.read_u16_le()?;
    let drs = cursor.read_u8()?;
    let rev_lights_percent = cursor.read_u8()?;
    cursor.skip(layout.car_telemetry_data_len - MAPPED_PLAYER_FIELDS_LEN)?;

    Ok(CarTelemetrySample {
        speed_kph,
        throttle,
        brake,
        gear,
        engine_rpm,
        drs,
        rev_lights_percent,
    })
}

fn validate_header_and_length(
    header: &PacketHeader,
    datagram_len: usize,
    layout: CarTelemetryLayout,
) -> Result<(), DecodeError> {
    if header.packet_id != CAR_TELEMETRY_PACKET_ID {
        return Err(DecodeError::UnexpectedPacketId {
            expected: CAR_TELEMETRY_PACKET_ID,
            actual: header.packet_id,
        });
    }
    if header.packet_version != CAR_TELEMETRY_PACKET_VERSION {
        return Err(DecodeError::UnsupportedPacketVersion {
            packet_id: header.packet_id,
            expected: CAR_TELEMETRY_PACKET_VERSION,
            actual: header.packet_version,
        });
    }
    if datagram_len != layout.packet_len {
        return Err(DecodeError::InvalidPacketLength {
            packet_id: header.packet_id,
            expected: layout.packet_len,
            actual: datagram_len,
        });
    }
    Ok(())
}
