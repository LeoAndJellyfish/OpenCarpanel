use crate::{Cursor, DecodeError, PacketHeader};

use super::{CAR_TELEMETRY2_PACKET_ID, F1Layout, player_entry, validate_packet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub(crate) struct CarTelemetry2Sample {
    pub(crate) active_aero_mode: u8,
    pub(crate) active_aero_available: bool,
    pub(crate) active_aero_activation_distance_m: u16,
    pub(crate) overtake_available: bool,
    pub(crate) overtake_active: bool,
    pub(crate) overtake_activation_distance_m: u16,
    pub(crate) regulations_2026: bool,
    pub(crate) driving_wrong_way: bool,
}

pub(crate) fn decode_player_car_telemetry2(
    header: &PacketHeader,
    payload: &[u8],
    datagram_len: usize,
    layout: F1Layout,
) -> Result<CarTelemetry2Sample, DecodeError> {
    let Some((entry_len, packet_len)) = layout.car_telemetry2 else {
        return Err(DecodeError::UnexpectedPacketId {
            expected: CAR_TELEMETRY2_PACKET_ID,
            actual: header.packet_id,
        });
    };
    validate_packet(header, datagram_len, CAR_TELEMETRY2_PACKET_ID, packet_len)?;
    let entry = player_entry(header, payload, layout.car_count, entry_len)?;
    let mut cursor = Cursor::new(entry);
    let active_aero_mode = cursor.read_u8()?;
    if active_aero_mode > 1 {
        return Err(DecodeError::InvalidEnumValue {
            field: "active_aero_mode",
            actual: active_aero_mode,
        });
    }
    let active_aero_available = read_bool(&mut cursor, "active_aero_available")?;
    let active_aero_activation_distance_m = cursor.read_u16_le()?;
    let overtake_available = read_bool(&mut cursor, "overtake_available")?;
    let overtake_active = read_bool(&mut cursor, "overtake_active")?;
    let overtake_activation_distance_m = cursor.read_u16_le()?;
    let regulations_2026 = read_bool(&mut cursor, "2026_regulations")?;
    let driving_wrong_way = read_bool(&mut cursor, "driving_wrong_way")?;
    debug_assert!(cursor.remaining().is_empty());

    Ok(CarTelemetry2Sample {
        active_aero_mode,
        active_aero_available,
        active_aero_activation_distance_m,
        overtake_available,
        overtake_active,
        overtake_activation_distance_m,
        regulations_2026,
        driving_wrong_way,
    })
}

fn read_bool(cursor: &mut Cursor<'_>, field: &'static str) -> Result<bool, DecodeError> {
    let value = cursor.read_u8()?;
    match value {
        0 => Ok(false),
        1 => Ok(true),
        actual => Err(DecodeError::InvalidEnumValue { field, actual }),
    }
}
