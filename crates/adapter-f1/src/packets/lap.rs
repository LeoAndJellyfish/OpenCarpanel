use crate::{Cursor, DecodeError, PacketHeader};

use super::{F1Layout, LAP_DATA_LEN, LAP_DATA_PACKET_ID, player_entry, validate_packet};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LapSample {
    pub(crate) last_lap_time_ms: u32,
    pub(crate) current_lap_time_ms: u32,
    pub(crate) sector1_time_ms_part: u16,
    pub(crate) sector1_time_minutes_part: u8,
    pub(crate) sector2_time_ms_part: u16,
    pub(crate) sector2_time_minutes_part: u8,
    pub(crate) delta_to_car_in_front_ms_part: u16,
    pub(crate) delta_to_car_in_front_minutes_part: u8,
    pub(crate) delta_to_race_leader_ms_part: u16,
    pub(crate) delta_to_race_leader_minutes_part: u8,
    pub(crate) lap_distance_m: f32,
    pub(crate) total_distance_m: f32,
    pub(crate) safety_car_delta_s: f32,
    pub(crate) car_position: u8,
    pub(crate) current_lap_number: u8,
    pub(crate) pit_status: u8,
    pub(crate) pit_stops: u8,
    pub(crate) sector: u8,
    pub(crate) current_lap_invalid: u8,
    pub(crate) penalties_seconds: u8,
    pub(crate) warnings: u8,
    pub(crate) corner_cutting_warnings: u8,
    pub(crate) unserved_drive_through_penalties: u8,
    pub(crate) unserved_stop_go_penalties: u8,
    pub(crate) grid_position: u8,
    pub(crate) driver_status: u8,
    pub(crate) result_status: u8,
    pub(crate) pit_lane_timer_active: u8,
    pub(crate) pit_lane_time_ms: u16,
    pub(crate) pit_stop_time_ms: u16,
    pub(crate) pit_stop_should_serve_penalty: u8,
}

pub(crate) fn decode_player_lap(
    header: &PacketHeader,
    payload: &[u8],
    datagram_len: usize,
    layout: F1Layout,
) -> Result<LapSample, DecodeError> {
    validate_packet(
        header,
        datagram_len,
        LAP_DATA_PACKET_ID,
        layout.lap_packet_len,
    )?;
    let entry = player_entry(header, payload, layout.car_count, LAP_DATA_LEN)?;
    let mut cursor = Cursor::new(entry);
    let sample = LapSample {
        last_lap_time_ms: cursor.read_u32_le()?,
        current_lap_time_ms: cursor.read_u32_le()?,
        sector1_time_ms_part: cursor.read_u16_le()?,
        sector1_time_minutes_part: cursor.read_u8()?,
        sector2_time_ms_part: cursor.read_u16_le()?,
        sector2_time_minutes_part: cursor.read_u8()?,
        delta_to_car_in_front_ms_part: cursor.read_u16_le()?,
        delta_to_car_in_front_minutes_part: cursor.read_u8()?,
        delta_to_race_leader_ms_part: cursor.read_u16_le()?,
        delta_to_race_leader_minutes_part: cursor.read_u8()?,
        lap_distance_m: cursor.read_f32_le()?,
        total_distance_m: cursor.read_f32_le()?,
        safety_car_delta_s: cursor.read_f32_le()?,
        car_position: cursor.read_u8()?,
        current_lap_number: cursor.read_u8()?,
        pit_status: cursor.read_u8()?,
        pit_stops: cursor.read_u8()?,
        sector: cursor.read_u8()?,
        current_lap_invalid: cursor.read_u8()?,
        penalties_seconds: cursor.read_u8()?,
        warnings: cursor.read_u8()?,
        corner_cutting_warnings: cursor.read_u8()?,
        unserved_drive_through_penalties: cursor.read_u8()?,
        unserved_stop_go_penalties: cursor.read_u8()?,
        grid_position: cursor.read_u8()?,
        driver_status: cursor.read_u8()?,
        result_status: cursor.read_u8()?,
        pit_lane_timer_active: cursor.read_u8()?,
        pit_lane_time_ms: cursor.read_u16_le()?,
        pit_stop_time_ms: cursor.read_u16_le()?,
        pit_stop_should_serve_penalty: cursor.read_u8()?,
    };
    let speed_trap_fastest_speed = cursor.read_f32_le()?;
    if !sample.lap_distance_m.is_finite()
        || !sample.total_distance_m.is_finite()
        || !sample.safety_car_delta_s.is_finite()
        || !speed_trap_fastest_speed.is_finite()
    {
        return Err(DecodeError::InvalidNumericValue {
            field: "lap_data",
            value: f32::NAN,
        });
    }
    let _speed_trap_fastest_lap = cursor.read_u8()?;
    debug_assert!(cursor.remaining().is_empty());
    Ok(sample)
}
