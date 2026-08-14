use crate::{Cursor, DecodeError, PacketHeader};

use super::{CAR_DAMAGE_PACKET_ID, F1Layout, player_entry, validate_packet};

#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(clippy::struct_excessive_bools)]
pub(crate) struct CarDamageSample {
    /// Official array order is rear-left, rear-right, front-left, front-right.
    pub(crate) tyre_wear_percent: [f32; 4],
    pub(crate) tyre_damage_percent: [u8; 4],
    pub(crate) brake_damage_percent: [u8; 4],
    pub(crate) tyre_blister_percent: Option<[u8; 4]>,
    pub(crate) front_left_wing_percent: u8,
    pub(crate) front_right_wing_percent: u8,
    pub(crate) rear_wing_percent: u8,
    pub(crate) floor_percent: u8,
    pub(crate) diffuser_percent: u8,
    pub(crate) sidepod_percent: u8,
    pub(crate) drs_fault: bool,
    pub(crate) ers_fault: bool,
    pub(crate) gearbox_percent: u8,
    pub(crate) engine_percent: u8,
    pub(crate) engine_mguh_wear_percent: u8,
    pub(crate) engine_es_wear_percent: u8,
    pub(crate) engine_ce_wear_percent: u8,
    pub(crate) engine_ice_wear_percent: u8,
    pub(crate) engine_mguk_wear_percent: u8,
    pub(crate) engine_tc_wear_percent: u8,
    pub(crate) engine_blown: bool,
    pub(crate) engine_seized: bool,
}

#[allow(clippy::similar_names)]
pub(crate) fn decode_player_car_damage(
    header: &PacketHeader,
    payload: &[u8],
    datagram_len: usize,
    layout: F1Layout,
) -> Result<CarDamageSample, DecodeError> {
    validate_packet(
        header,
        datagram_len,
        CAR_DAMAGE_PACKET_ID,
        layout.car_damage_packet_len,
    )?;
    let entry = player_entry(
        header,
        payload,
        layout.car_count,
        layout.car_damage_data_len,
    )?;
    let mut cursor = Cursor::new(entry);
    let mut tyre_wear_percent = [0.0; 4];
    let mut tyre_damage_percent = [0; 4];
    let mut brake_damage_percent = [0; 4];
    for value in &mut tyre_wear_percent {
        *value = cursor.read_f32_le()?;
        validate_percent("tyres_wear", *value)?;
    }
    for value in &mut tyre_damage_percent {
        *value = cursor.read_u8()?;
        validate_u8_percent("tyres_damage", *value)?;
    }
    for value in &mut brake_damage_percent {
        *value = cursor.read_u8()?;
        validate_u8_percent("brakes_damage", *value)?;
    }
    let tyre_blister_percent = if layout.car_damage_data_len == super::F1_25_CAR_DAMAGE_DATA_LEN {
        let mut values = [0; 4];
        for value in &mut values {
            *value = cursor.read_u8()?;
            validate_u8_percent("tyre_blisters", *value)?;
        }
        Some(values)
    } else {
        None
    };

    let front_left_wing_percent = read_percent(&mut cursor, "front_left_wing_damage")?;
    let front_right_wing_percent = read_percent(&mut cursor, "front_right_wing_damage")?;
    let rear_wing_percent = read_percent(&mut cursor, "rear_wing_damage")?;
    let floor_percent = read_percent(&mut cursor, "floor_damage")?;
    let diffuser_percent = read_percent(&mut cursor, "diffuser_damage")?;
    let sidepod_percent = read_percent(&mut cursor, "sidepod_damage")?;
    let drs_fault = read_bool(&mut cursor, "drs_fault")?;
    let ers_fault = read_bool(&mut cursor, "ers_fault")?;
    let gearbox_percent = read_percent(&mut cursor, "gear_box_damage")?;
    let engine_percent = read_percent(&mut cursor, "engine_damage")?;
    let engine_mguh_wear_percent = read_percent(&mut cursor, "engine_mguh_wear")?;
    let engine_es_wear_percent = read_percent(&mut cursor, "engine_es_wear")?;
    let engine_ce_wear_percent = read_percent(&mut cursor, "engine_ce_wear")?;
    let engine_ice_wear_percent = read_percent(&mut cursor, "engine_ice_wear")?;
    let engine_mguk_wear_percent = read_percent(&mut cursor, "engine_mguk_wear")?;
    let engine_tc_wear_percent = read_percent(&mut cursor, "engine_tc_wear")?;
    let engine_blown = read_bool(&mut cursor, "engine_blown")?;
    let engine_seized = read_bool(&mut cursor, "engine_seized")?;
    debug_assert!(cursor.remaining().is_empty());

    Ok(CarDamageSample {
        tyre_wear_percent,
        tyre_damage_percent,
        brake_damage_percent,
        tyre_blister_percent,
        front_left_wing_percent,
        front_right_wing_percent,
        rear_wing_percent,
        floor_percent,
        diffuser_percent,
        sidepod_percent,
        drs_fault,
        ers_fault,
        gearbox_percent,
        engine_percent,
        engine_mguh_wear_percent,
        engine_es_wear_percent,
        engine_ce_wear_percent,
        engine_ice_wear_percent,
        engine_mguk_wear_percent,
        engine_tc_wear_percent,
        engine_blown,
        engine_seized,
    })
}

fn validate_percent(field: &'static str, value: f32) -> Result<(), DecodeError> {
    if value.is_finite() && (0.0..=100.0).contains(&value) {
        Ok(())
    } else {
        Err(DecodeError::InvalidNumericValue { field, value })
    }
}

fn validate_u8_percent(field: &'static str, value: u8) -> Result<(), DecodeError> {
    if value <= 100 {
        Ok(())
    } else {
        Err(DecodeError::InvalidEnumValue {
            field,
            actual: value,
        })
    }
}

fn read_percent(cursor: &mut Cursor<'_>, field: &'static str) -> Result<u8, DecodeError> {
    let value = cursor.read_u8()?;
    validate_u8_percent(field, value)?;
    Ok(value)
}

fn read_bool(cursor: &mut Cursor<'_>, field: &'static str) -> Result<bool, DecodeError> {
    let value = cursor.read_u8()?;
    match value {
        0 => Ok(false),
        1 => Ok(true),
        actual => Err(DecodeError::InvalidEnumValue { field, actual }),
    }
}
