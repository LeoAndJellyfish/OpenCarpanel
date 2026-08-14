use crate::{Cursor, DecodeError, PacketHeader};

use super::{CAR_STATUS_PACKET_ID, F1Layout, player_entry, validate_packet};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct CarStatusSample {
    pub(crate) pit_limiter_active: bool,
    pub(crate) fuel_kg: f32,
    pub(crate) fuel_capacity_kg: f32,
    pub(crate) fuel_remaining_laps: f32,
    pub(crate) rpm_max: u16,
    pub(crate) drs_allowed: bool,
    pub(crate) actual_tyre_compound: u8,
    pub(crate) visual_tyre_compound: u8,
    pub(crate) tyre_age_laps: u8,
    pub(crate) vehicle_fia_flag: i8,
    pub(crate) ers_store_energy_j: f32,
}

#[allow(clippy::similar_names)]
pub(crate) fn decode_player_car_status(
    header: &PacketHeader,
    payload: &[u8],
    datagram_len: usize,
    layout: F1Layout,
) -> Result<CarStatusSample, DecodeError> {
    validate_packet(
        header,
        datagram_len,
        CAR_STATUS_PACKET_ID,
        layout.car_status_packet_len,
    )?;
    let entry = player_entry(
        header,
        payload,
        layout.car_count,
        layout.car_status_data_len,
    )?;
    let mut cursor = Cursor::new(entry);
    let _traction_control = cursor.read_u8()?;
    let _anti_lock_brakes = cursor.read_u8()?;
    let _fuel_mix = cursor.read_u8()?;
    let _front_brake_bias = cursor.read_u8()?;
    let pit_limiter = cursor.read_u8()?;
    let fuel_kg = cursor.read_f32_le()?;
    let fuel_capacity_kg = cursor.read_f32_le()?;
    let fuel_remaining_laps = cursor.read_f32_le()?;
    let rpm_max = cursor.read_u16_le()?;
    let _idle_rpm = cursor.read_u16_le()?;
    let _max_gears = cursor.read_u8()?;
    let drs_allowed = cursor.read_u8()?;
    let _drs_activation_distance = cursor.read_u16_le()?;
    let actual_tyre_compound = cursor.read_u8()?;
    let visual_tyre_compound = cursor.read_u8()?;
    let tyre_age_laps = cursor.read_u8()?;
    let vehicle_fia_flag = cursor.read_i8()?;
    let engine_power_ice = cursor.read_f32_le()?;
    let engine_power_mguk = cursor.read_f32_le()?;
    let ers_store_energy_j = cursor.read_f32_le()?;
    let _ers_deploy_mode = cursor.read_u8()?;
    let ers_harvested_mguk = cursor.read_f32_le()?;
    let ers_harvested_mguh = cursor.read_f32_le()?;
    let ers_harvest_limit = if layout.car_status_data_len == super::F1_25_2026_CAR_STATUS_DATA_LEN {
        Some(cursor.read_f32_le()?)
    } else {
        None
    };
    let ers_deployed = cursor.read_f32_le()?;
    let _network_paused = cursor.read_u8()?;
    debug_assert!(cursor.remaining().is_empty());

    if pit_limiter > 1 {
        return Err(DecodeError::InvalidEnumValue {
            field: "pit_limiter_status",
            actual: pit_limiter,
        });
    }
    if drs_allowed > 1 {
        return Err(DecodeError::InvalidEnumValue {
            field: "drs_allowed",
            actual: drs_allowed,
        });
    }
    for (field, value) in [
        ("fuel_in_tank", fuel_kg),
        ("fuel_capacity", fuel_capacity_kg),
        ("fuel_remaining_laps", fuel_remaining_laps),
        ("engine_power_ice", engine_power_ice),
        ("engine_power_mguk", engine_power_mguk),
        ("ers_store_energy", ers_store_energy_j),
        ("ers_harvested_mguk", ers_harvested_mguk),
        ("ers_harvested_mguh", ers_harvested_mguh),
        ("ers_deployed", ers_deployed),
    ]
    .into_iter()
    .chain(ers_harvest_limit.map(|value| ("ers_harvest_limit", value)))
    {
        if !value.is_finite() {
            return Err(DecodeError::InvalidNumericValue { field, value });
        }
    }

    Ok(CarStatusSample {
        pit_limiter_active: pit_limiter == 1,
        fuel_kg,
        fuel_capacity_kg,
        fuel_remaining_laps,
        rpm_max,
        drs_allowed: drs_allowed == 1,
        actual_tyre_compound,
        visual_tyre_compound,
        tyre_age_laps,
        vehicle_fia_flag,
        ers_store_energy_j,
    })
}
