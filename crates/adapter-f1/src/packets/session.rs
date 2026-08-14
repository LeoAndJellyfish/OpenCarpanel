use crate::{Cursor, DecodeError, PacketHeader};

use super::{F1Layout, SESSION_PACKET_ID, validate_packet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SessionSample {
    pub(crate) weather: u8,
    pub(crate) track_temperature_c: i8,
    pub(crate) air_temperature_c: i8,
    pub(crate) total_laps: u8,
    pub(crate) track_length_m: u16,
    pub(crate) session_type: u8,
    pub(crate) track_id: i8,
    pub(crate) session_time_left_s: u16,
    pub(crate) pit_speed_limit_kph: u8,
    pub(crate) safety_car_status: u8,
}

pub(crate) fn decode_session(
    header: &PacketHeader,
    payload: &[u8],
    datagram_len: usize,
    layout: F1Layout,
) -> Result<SessionSample, DecodeError> {
    validate_packet(
        header,
        datagram_len,
        SESSION_PACKET_ID,
        layout.session_packet_len,
    )?;
    let mut cursor = Cursor::new(payload);
    let weather = cursor.read_u8()?;
    let track_temperature_c = cursor.read_i8()?;
    let air_temperature_c = cursor.read_i8()?;
    let total_laps = cursor.read_u8()?;
    let track_length_m = cursor.read_u16_le()?;
    let session_type = cursor.read_u8()?;
    let track_id = cursor.read_i8()?;
    let _formula = cursor.read_u8()?;
    let session_time_left_s = cursor.read_u16_le()?;
    let _session_duration_s = cursor.read_u16_le()?;
    let pit_speed_limit_kph = cursor.read_u8()?;
    let _game_paused = cursor.read_u8()?;
    let _is_spectating = cursor.read_u8()?;
    let _spectator_car_index = cursor.read_u8()?;
    let _sli_pro_native_support = cursor.read_u8()?;
    let marshal_zone_count = cursor.read_u8()?;
    if marshal_zone_count > 21 {
        return Err(DecodeError::InvalidEnumValue {
            field: "num_marshal_zones",
            actual: marshal_zone_count,
        });
    }
    cursor.skip(21 * 5)?;
    let safety_car_status = cursor.read_u8()?;

    Ok(SessionSample {
        weather,
        track_temperature_c,
        air_temperature_c,
        total_laps,
        track_length_m,
        session_type,
        track_id,
        session_time_left_s,
        pit_speed_limit_kph,
        safety_car_status,
    })
}
