mod car_damage;
mod car_status;
mod car_telemetry;
mod car_telemetry2;
mod event;
mod lap;
mod session;

pub(crate) use car_damage::{CarDamageSample, decode_player_car_damage};
pub(crate) use car_status::{CarStatusSample, decode_player_car_status};
pub(crate) use car_telemetry::{CarTelemetrySample, decode_player_car_telemetry};
pub(crate) use car_telemetry2::{CarTelemetry2Sample, decode_player_car_telemetry2};
pub(crate) use event::{EventSample, decode_event};
pub(crate) use lap::{LapSample, decode_player_lap};
pub(crate) use session::{SessionSample, decode_session};

use crate::{DecodeError, PacketHeader};

/// Packet id assigned to Session Data by the supported F1 specifications.
pub const SESSION_PACKET_ID: u8 = 1;
/// Packet id assigned to Lap Data by the supported F1 specifications.
pub const LAP_DATA_PACKET_ID: u8 = 2;
/// Packet id assigned to Event Data by the supported F1 specifications.
pub const EVENT_PACKET_ID: u8 = 3;
/// Packet id assigned to Car Telemetry by the supported F1 specifications.
pub const CAR_TELEMETRY_PACKET_ID: u8 = 6;
/// Packet id assigned to Car Status by the supported F1 specifications.
pub const CAR_STATUS_PACKET_ID: u8 = 7;
/// Packet id assigned to Car Damage by the supported F1 specifications.
pub const CAR_DAMAGE_PACKET_ID: u8 = 10;
/// Packet id assigned to Car Telemetry 2 by the 2026 Season Pack specification.
pub const CAR_TELEMETRY2_PACKET_ID: u8 = 16;

/// Packet version implemented for every supported packet type.
pub const PACKET_VERSION: u8 = 1;
/// Car Telemetry packet version implemented for compatibility with the original public API.
pub const CAR_TELEMETRY_PACKET_VERSION: u8 = PACKET_VERSION;

/// Number of car entries in F1 24 and original F1 25 packets.
pub const F1_CAR_COUNT: usize = 22;
/// Number of car entries in F1 25 2026 Season Pack packets.
pub const F1_25_2026_CAR_COUNT: usize = 24;

/// Packed byte length of one F1 24 or original F1 25 `CarTelemetryData` entry.
pub const CAR_TELEMETRY_DATA_LEN: usize = 60;
/// Packed byte length of one complete F1 24 or original F1 25 Car Telemetry datagram.
pub const CAR_TELEMETRY_PACKET_LEN: usize = 1_352;
/// Packed byte length of one 2026 Season Pack `CarTelemetryData` entry.
pub const F1_25_2026_CAR_TELEMETRY_DATA_LEN: usize = 59;
/// Packed byte length of one complete 2026 Season Pack Car Telemetry datagram.
pub const F1_25_2026_CAR_TELEMETRY_PACKET_LEN: usize = 1_448;

/// Packed byte length of a F1 24/original F1 25 Session datagram.
pub const SESSION_PACKET_LEN: usize = 753;
/// Packed byte length of a 2026 Season Pack Session datagram.
pub const F1_25_2026_SESSION_PACKET_LEN: usize = 926;
/// Packed byte length of one Lap Data entry.
pub const LAP_DATA_LEN: usize = 57;
/// Packed byte length of a F1 24/original F1 25 Lap Data datagram.
pub const LAP_DATA_PACKET_LEN: usize = 1_285;
/// Packed byte length of a 2026 Season Pack Lap Data datagram.
pub const F1_25_2026_LAP_DATA_PACKET_LEN: usize = 1_399;
/// Packed byte length of every supported Event datagram.
pub const EVENT_PACKET_LEN: usize = 45;
/// Packed byte length of one F1 24/original F1 25 Car Status entry.
pub const CAR_STATUS_DATA_LEN: usize = 55;
/// Packed byte length of a F1 24/original F1 25 Car Status datagram.
pub const CAR_STATUS_PACKET_LEN: usize = 1_239;
/// Packed byte length of one 2026 Season Pack Car Status entry.
pub const F1_25_2026_CAR_STATUS_DATA_LEN: usize = 59;
/// Packed byte length of a 2026 Season Pack Car Status datagram.
pub const F1_25_2026_CAR_STATUS_PACKET_LEN: usize = 1_445;
/// Packed byte length of one F1 24 Car Damage entry.
pub const F1_24_CAR_DAMAGE_DATA_LEN: usize = 42;
/// Packed byte length of a F1 24 Car Damage datagram.
pub const F1_24_CAR_DAMAGE_PACKET_LEN: usize = 953;
/// Packed byte length of one F1 25 Car Damage entry.
pub const F1_25_CAR_DAMAGE_DATA_LEN: usize = 46;
/// Packed byte length of an original F1 25 Car Damage datagram.
pub const F1_25_CAR_DAMAGE_PACKET_LEN: usize = 1_041;
/// Packed byte length of a 2026 Season Pack Car Damage datagram.
pub const F1_25_2026_CAR_DAMAGE_PACKET_LEN: usize = 1_133;
/// Packed byte length of one 2026 Season Pack Car Telemetry 2 entry.
pub const CAR_TELEMETRY2_DATA_LEN: usize = 10;
/// Packed byte length of a 2026 Season Pack Car Telemetry 2 datagram.
pub const CAR_TELEMETRY2_PACKET_LEN: usize = 269;

/// Exact packed layouts selected only after validating `packetFormat`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct F1Layout {
    pub(crate) packet_format: u16,
    pub(crate) car_count: usize,
    pub(crate) session_packet_len: usize,
    pub(crate) lap_packet_len: usize,
    pub(crate) car_telemetry_data_len: usize,
    pub(crate) car_telemetry_packet_len: usize,
    pub(crate) car_status_data_len: usize,
    pub(crate) car_status_packet_len: usize,
    pub(crate) car_damage_data_len: usize,
    pub(crate) car_damage_packet_len: usize,
    pub(crate) car_telemetry2: Option<(usize, usize)>,
}

pub(crate) fn validate_packet(
    header: &PacketHeader,
    datagram_len: usize,
    packet_id: u8,
    expected_len: usize,
) -> Result<(), DecodeError> {
    if header.packet_id != packet_id {
        return Err(DecodeError::UnexpectedPacketId {
            expected: packet_id,
            actual: header.packet_id,
        });
    }
    if header.packet_version != PACKET_VERSION {
        return Err(DecodeError::UnsupportedPacketVersion {
            packet_id: header.packet_id,
            expected: PACKET_VERSION,
            actual: header.packet_version,
        });
    }
    if datagram_len != expected_len {
        return Err(DecodeError::InvalidPacketLength {
            packet_id: header.packet_id,
            expected: expected_len,
            actual: datagram_len,
        });
    }
    Ok(())
}

pub(crate) fn player_entry<'a>(
    header: &PacketHeader,
    payload: &'a [u8],
    car_count: usize,
    entry_len: usize,
) -> Result<&'a [u8], DecodeError> {
    let player_index = usize::from(header.player_car_index);
    if player_index >= car_count {
        return Err(DecodeError::InvalidPlayerIndex {
            index: header.player_car_index,
            car_count,
        });
    }
    let start = player_index * entry_len;
    let end = start + entry_len;
    payload.get(start..end).ok_or(DecodeError::UnexpectedEnd {
        offset: start,
        needed: entry_len,
        remaining: payload.len().saturating_sub(start),
    })
}
