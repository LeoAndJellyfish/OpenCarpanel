use crate::{Cursor, DecodeError, PacketHeader};

use super::{CAR_TELEMETRY_PACKET_ID, F1Layout, player_entry, validate_packet};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct CarTelemetrySample {
    pub(crate) speed_kph: u16,
    pub(crate) throttle: f32,
    pub(crate) brake: f32,
    pub(crate) gear: i8,
    pub(crate) engine_rpm: u16,
    pub(crate) drs_active: bool,
    pub(crate) rev_lights_percent: u8,
    /// Official array order is rear-left, rear-right, front-left, front-right.
    pub(crate) tyre_surface_temperature_c: [u8; 4],
    /// Official array order is rear-left, rear-right, front-left, front-right.
    pub(crate) tyre_inner_temperature_c: [u8; 4],
    /// Official array order is rear-left, rear-right, front-left, front-right.
    pub(crate) tyre_pressure_psi: [f32; 4],
}

pub(crate) fn decode_player_car_telemetry(
    header: &PacketHeader,
    payload: &[u8],
    datagram_len: usize,
    layout: F1Layout,
) -> Result<CarTelemetrySample, DecodeError> {
    validate_packet(
        header,
        datagram_len,
        CAR_TELEMETRY_PACKET_ID,
        layout.car_telemetry_packet_len,
    )?;
    let entry = player_entry(
        header,
        payload,
        layout.car_count,
        layout.car_telemetry_data_len,
    )?;
    let mut cursor = Cursor::new(entry);
    let speed_kph = cursor.read_u16_le()?;
    let throttle = cursor.read_f32_le()?;
    let _steer = cursor.read_f32_le()?;
    let brake = cursor.read_f32_le()?;
    let _clutch = cursor.read_u8()?;
    let gear = cursor.read_i8()?;
    let engine_rpm = cursor.read_u16_le()?;
    let drs = cursor.read_u8()?;
    if drs > 1 {
        return Err(DecodeError::InvalidEnumValue {
            field: "drs",
            actual: drs,
        });
    }
    let rev_lights_percent = cursor.read_u8()?;
    let _rev_lights_bit_value = cursor.read_u16_le()?;
    for _ in 0..4 {
        let _brake_temperature = cursor.read_u16_le()?;
    }

    let mut tyre_surface_temperature_c = [0; 4];
    let mut tyre_inner_temperature_c = [0; 4];
    let mut tyre_pressure_psi = [0.0; 4];
    for value in &mut tyre_surface_temperature_c {
        *value = cursor.read_u8()?;
    }
    for value in &mut tyre_inner_temperature_c {
        *value = cursor.read_u8()?;
    }
    if layout.car_telemetry_data_len == super::F1_25_2026_CAR_TELEMETRY_DATA_LEN {
        let _engine_temperature = cursor.read_u8()?;
    } else {
        let _engine_temperature = cursor.read_u16_le()?;
    }
    for value in &mut tyre_pressure_psi {
        *value = cursor.read_f32_le()?;
        if !value.is_finite() || *value < 0.0 {
            return Err(DecodeError::InvalidNumericValue {
                field: "tyres_pressure",
                value: *value,
            });
        }
    }
    for _ in 0..4 {
        let _surface_type = cursor.read_u8()?;
    }
    debug_assert!(cursor.remaining().is_empty());

    Ok(CarTelemetrySample {
        speed_kph,
        throttle,
        brake,
        gear,
        engine_rpm,
        drs_active: drs == 1,
        rev_lights_percent,
        tyre_surface_temperature_c,
        tyre_inner_temperature_c,
        tyre_pressure_psi,
    })
}
