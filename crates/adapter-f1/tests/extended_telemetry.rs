use std::error::Error;

use opensimdash_adapter_api::{AdapterOutput, GameAdapter};
use opensimdash_adapter_f1::{
    CAR_STATUS_DATA_LEN, CAR_STATUS_PACKET_LEN, CAR_TELEMETRY2_DATA_LEN, CAR_TELEMETRY2_PACKET_ID,
    CAR_TELEMETRY2_PACKET_LEN, F1_24_CAR_DAMAGE_DATA_LEN, F1_24_CAR_DAMAGE_PACKET_LEN,
    F1_24_PACKET_FORMAT, F1_24Adapter, F1_25_2026_CAR_COUNT, F1_25_2026_CAR_DAMAGE_PACKET_LEN,
    F1_25_2026_CAR_STATUS_DATA_LEN, F1_25_2026_CAR_STATUS_PACKET_LEN,
    F1_25_2026_CAR_TELEMETRY_DATA_LEN, F1_25_2026_CAR_TELEMETRY_PACKET_LEN,
    F1_25_2026_LAP_DATA_PACKET_LEN, F1_25_2026_PACKET_FORMAT, F1_25_2026_SESSION_PACKET_LEN,
    F1_25_CAR_DAMAGE_DATA_LEN, F1_25_CAR_DAMAGE_PACKET_LEN, F1_25_PACKET_FORMAT, F1_25Adapter,
    LAP_DATA_LEN, LAP_DATA_PACKET_ID, LAP_DATA_PACKET_LEN, SESSION_PACKET_LEN,
};
use opensimdash_telemetry_core::{
    ActiveAeroMode, DrsState, MonotonicTimestamp, Normalized, PitStatus, RaceFlag, SafetyCarStatus,
    WeatherCondition,
};

const HEADER_LEN: usize = 29;
const PLAYER: u8 = 23;

fn packet(packet_id: u8, length: usize, player: u8) -> Vec<u8> {
    let mut bytes = vec![0; length];
    write_u16(&mut bytes, 0, F1_25_2026_PACKET_FORMAT);
    bytes[2] = 26;
    bytes[3] = 1;
    bytes[4] = 0;
    bytes[5] = 1;
    bytes[6] = packet_id;
    bytes[7..15].copy_from_slice(&0x0102_0304_0506_0708_u64.to_le_bytes());
    write_u32(&mut bytes, 19, 41);
    write_u32(&mut bytes, 23, 42);
    bytes[27] = player;
    bytes[28] = u8::MAX;
    bytes
}

fn legacy_packet(format: u16, game_year: u8, packet_id: u8, length: usize) -> Vec<u8> {
    let mut bytes = vec![0; length];
    write_u16(&mut bytes, 0, format);
    bytes[2] = game_year;
    bytes[3] = 1;
    bytes[5] = 1;
    bytes[6] = packet_id;
    bytes[7..15].copy_from_slice(&0x1112_1314_1516_1718_u64.to_le_bytes());
    write_u32(&mut bytes, 19, 11);
    write_u32(&mut bytes, 23, 12);
    bytes[28] = u8::MAX;
    bytes
}

fn write_u16(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn write_u32(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn write_f32(bytes: &mut [u8], offset: usize, value: f32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn session_packet() -> Vec<u8> {
    let mut bytes = packet(1, F1_25_2026_SESSION_PACKET_LEN, PLAYER);
    let p = HEADER_LEN;
    bytes[p] = 3;
    bytes[p + 1] = 41_i8.to_le_bytes()[0];
    bytes[p + 2] = 27_i8.to_le_bytes()[0];
    bytes[p + 3] = 57;
    write_u16(&mut bytes, p + 4, 5_412);
    bytes[p + 6] = 10;
    bytes[p + 7] = 4;
    bytes[p + 8] = 13;
    write_u16(&mut bytes, p + 9, 1_234);
    write_u16(&mut bytes, p + 11, 3_600);
    bytes[p + 13] = 80;
    bytes[p + 18] = 0;
    bytes[p + 124] = 2;
    bytes
}

fn lap_packet(current_lap: u8, last_lap_ms: u32) -> Vec<u8> {
    let mut bytes = packet(LAP_DATA_PACKET_ID, F1_25_2026_LAP_DATA_PACKET_LEN, PLAYER);
    let p = HEADER_LEN + usize::from(PLAYER) * LAP_DATA_LEN;
    write_u32(&mut bytes, p, last_lap_ms);
    write_u32(&mut bytes, p + 4, 45_321);
    write_u16(&mut bytes, p + 8, 31_234);
    bytes[p + 10] = 0;
    write_u16(&mut bytes, p + 11, 30_123);
    write_u16(&mut bytes, p + 14, 420);
    write_u16(&mut bytes, p + 17, 1_250);
    write_f32(&mut bytes, p + 20, 2_340.5);
    write_f32(&mut bytes, p + 24, 7_752.0);
    write_f32(&mut bytes, p + 28, -0.215);
    bytes[p + 32] = 5;
    bytes[p + 33] = current_lap;
    bytes[p + 34] = 1;
    bytes[p + 35] = 2;
    bytes[p + 36] = 1;
    bytes[p + 37] = 1;
    bytes[p + 38] = 5;
    bytes[p + 39] = 3;
    bytes[p + 40] = 2;
    bytes[p + 41] = 1;
    bytes[p + 42] = 0;
    bytes[p + 43] = 7;
    bytes[p + 44] = 4;
    bytes[p + 45] = 2;
    bytes[p + 46] = 1;
    write_u16(&mut bytes, p + 47, 4_321);
    write_u16(&mut bytes, p + 49, 2_345);
    bytes[p + 51] = 1;
    write_f32(&mut bytes, p + 52, 321.0);
    bytes[p + 56] = current_lap;
    bytes
}

fn car_telemetry_packet() -> Vec<u8> {
    let mut bytes = packet(6, F1_25_2026_CAR_TELEMETRY_PACKET_LEN, PLAYER);
    let p = HEADER_LEN + usize::from(PLAYER) * F1_25_2026_CAR_TELEMETRY_DATA_LEN;
    write_u16(&mut bytes, p, 288);
    write_f32(&mut bytes, p + 2, 0.8);
    write_f32(&mut bytes, p + 10, 0.2);
    bytes[p + 15] = 7;
    write_u16(&mut bytes, p + 16, 12_345);
    bytes[p + 18] = 1;
    bytes[p + 19] = 92;
    bytes[p + 30..p + 34].copy_from_slice(&[91, 92, 93, 94]);
    bytes[p + 34..p + 38].copy_from_slice(&[81, 82, 83, 84]);
    bytes[p + 38] = 110;
    for (index, pressure) in [22.1, 22.2, 23.1, 23.2].into_iter().enumerate() {
        write_f32(&mut bytes, p + 39 + index * 4, pressure);
    }
    bytes
}

fn car_status_packet() -> Vec<u8> {
    let mut bytes = packet(7, F1_25_2026_CAR_STATUS_PACKET_LEN, PLAYER);
    let p = HEADER_LEN + usize::from(PLAYER) * F1_25_2026_CAR_STATUS_DATA_LEN;
    bytes[p + 4] = 1;
    write_f32(&mut bytes, p + 5, 31.5);
    write_f32(&mut bytes, p + 9, 110.0);
    write_f32(&mut bytes, p + 13, 18.25);
    write_u16(&mut bytes, p + 17, 15_000);
    write_u16(&mut bytes, p + 19, 4_000);
    bytes[p + 21] = 8;
    bytes[p + 22] = 1;
    bytes[p + 25] = 18;
    bytes[p + 26] = 16;
    bytes[p + 27] = 12;
    bytes[p + 28] = 2;
    write_f32(&mut bytes, p + 29, 500_000.0);
    write_f32(&mut bytes, p + 33, 120_000.0);
    write_f32(&mut bytes, p + 37, 3_250_000.0);
    bytes[p + 41] = 3;
    write_f32(&mut bytes, p + 42, 10_000.0);
    write_f32(&mut bytes, p + 46, 20_000.0);
    write_f32(&mut bytes, p + 50, 4_000_000.0);
    write_f32(&mut bytes, p + 54, 500_000.0);
    bytes
}

fn car_damage_packet() -> Vec<u8> {
    let mut bytes = packet(10, F1_25_2026_CAR_DAMAGE_PACKET_LEN, PLAYER);
    let p = HEADER_LEN + usize::from(PLAYER) * 46;
    for (index, wear) in [10.0, 20.0, 30.0, 40.0].into_iter().enumerate() {
        write_f32(&mut bytes, p + index * 4, wear);
    }
    bytes[p + 16..p + 20].copy_from_slice(&[1, 2, 3, 4]);
    bytes[p + 20..p + 24].copy_from_slice(&[5, 6, 7, 8]);
    bytes[p + 24..p + 28].copy_from_slice(&[9, 10, 11, 12]);
    bytes[p + 28..p + 34].copy_from_slice(&[13, 14, 15, 16, 17, 18]);
    bytes[p + 34] = 1;
    bytes[p + 35] = 0;
    bytes[p + 36..p + 44].copy_from_slice(&[19, 20, 21, 22, 23, 24, 25, 26]);
    bytes[p + 44] = 0;
    bytes[p + 45] = 1;
    bytes
}

fn car_telemetry2_packet() -> Vec<u8> {
    let mut bytes = packet(CAR_TELEMETRY2_PACKET_ID, CAR_TELEMETRY2_PACKET_LEN, PLAYER);
    let p = HEADER_LEN + usize::from(PLAYER) * CAR_TELEMETRY2_DATA_LEN;
    bytes[p] = 1;
    bytes[p + 1] = 1;
    write_u16(&mut bytes, p + 2, 125);
    bytes[p + 4] = 1;
    bytes[p + 5] = 1;
    write_u16(&mut bytes, p + 6, 250);
    bytes[p + 8] = 1;
    bytes[p + 9] = 0;
    bytes
}

fn penalty_event_packet() -> Vec<u8> {
    let mut bytes = packet(3, 45, PLAYER);
    bytes[HEADER_LEN..HEADER_LEN + 4].copy_from_slice(b"PENA");
    bytes[HEADER_LEN + 4..HEADER_LEN + 11].copy_from_slice(&[5, 12, PLAYER, 4, 10, 9, 2]);
    bytes
}

fn legacy_damage_packet(format: u16, game_year: u8, entry_len: usize, length: usize) -> Vec<u8> {
    let mut bytes = legacy_packet(format, game_year, 10, length);
    let p = HEADER_LEN;
    for (index, wear) in [10.0, 20.0, 30.0, 40.0].into_iter().enumerate() {
        write_f32(&mut bytes, p + index * 4, wear);
    }
    if entry_len == F1_25_CAR_DAMAGE_DATA_LEN {
        bytes[p + 24..p + 28].copy_from_slice(&[1, 2, 3, 4]);
    }
    assert_eq!((bytes.len() - HEADER_LEN) / 22, entry_len);
    bytes
}

#[test]
fn f1_2026_maps_session_lap_status_tyres_damage_and_aero() -> Result<(), Box<dyn Error>> {
    let mut adapter = F1_25Adapter::new()?;
    let mut output = AdapterOutput::default();
    let at = MonotonicTimestamp::from_micros(900);

    adapter.decode(&session_packet(), at, &mut output)?;
    let session = &output.updates[0];
    assert_eq!(session.session.track_id.as_deref(), Some("4"));
    assert_eq!(session.session.remaining_time_ms, Some(1_234_000));
    assert_eq!(session.session.total_laps, Some(57));
    assert_eq!(session.session.session_type.as_deref(), Some("race"));
    assert_eq!(
        session.session.safety_car_status,
        Some(SafetyCarStatus::Virtual)
    );
    assert_eq!(
        session.conditions.weather,
        Some(WeatherCondition::LightRain)
    );

    output.clear();
    adapter.decode(&lap_packet(9, 91_234), at, &mut output)?;
    let lap = &output.updates[0].lap;
    assert_eq!(lap.current, Some(9));
    assert_eq!(lap.position, Some(5));
    assert_eq!(lap.sector, Some(2));
    assert_eq!(lap.pit_status, Some(PitStatus::Pitting));
    assert_eq!(lap.penalties_seconds, Some(5));
    assert_eq!(lap.safety_car_delta_ms, Some(-215));

    output.clear();
    adapter.decode(&car_telemetry_packet(), at, &mut output)?;
    assert_eq!(output.updates[0].vehicle.drs, Some(DrsState::Active));
    assert_eq!(
        output.updates[0]
            .tyres
            .front_left
            .as_ref()
            .and_then(|tyre| tyre.surface_temperature_c),
        Some(93.0)
    );

    output.clear();
    adapter.decode(&car_status_packet(), at, &mut output)?;
    let status = &output.updates[0];
    assert_eq!(status.vehicle.fuel_kg, Some(31.5));
    assert_eq!(status.vehicle.rpm_max, Some(15_000));
    assert_eq!(status.vehicle.pit_limiter, Some(true));
    assert_eq!(status.session.race_flag, Some(RaceFlag::Blue));
    assert_eq!(status.tyres.actual_compound, Some(18));

    output.clear();
    adapter.decode(&car_damage_packet(), at, &mut output)?;
    let damage = &output.updates[0];
    assert_eq!(
        damage
            .tyres
            .front_left
            .as_ref()
            .and_then(|tyre| tyre.wear)
            .map(Normalized::get),
        Some(0.3)
    );
    assert_eq!(damage.damage.drs_fault, Some(true));
    assert_eq!(damage.damage.engine_seized, Some(true));

    output.clear();
    adapter.decode(&car_telemetry2_packet(), at, &mut output)?;
    let aero = &output.updates[0].aero;
    assert_eq!(aero.mode, Some(ActiveAeroMode::Straight));
    assert_eq!(aero.available, Some(true));
    assert_eq!(aero.overtake_active, Some(true));
    assert_eq!(aero.regulations_2026, Some(true));
    Ok(())
}

#[test]
fn f1_emits_structured_penalty_and_lap_completed_events() -> Result<(), Box<dyn Error>> {
    let mut adapter = F1_25Adapter::new()?;
    let mut output = AdapterOutput::default();
    let at = MonotonicTimestamp::from_micros(1_000);

    adapter.decode(&penalty_event_packet(), at, &mut output)?;
    assert_eq!(output.events[0].name, "penalty.issued");
    assert_eq!(output.events[0].data["isPlayer"], true);
    assert_eq!(output.events[0].data["timeSeconds"], 10);

    output.clear();
    adapter.decode(&lap_packet(9, 90_000), at, &mut output)?;
    assert!(output.events.is_empty());
    output.clear();
    let mut lap_ten = lap_packet(10, 89_500);
    write_u32(&mut lap_ten, 23, 43);
    adapter.decode(&lap_ten, at, &mut output)?;
    assert_eq!(output.events[0].name, "lap.completed");
    assert_eq!(output.events[0].data["lap"], 9);
    assert_eq!(output.events[0].data["lastTimeMs"], 89_500);

    output.clear();
    adapter.decode(&lap_packet(9, 90_000), at, &mut output)?;
    assert!(output.events.is_empty());
    output.clear();
    let mut lap_eleven = lap_packet(11, 88_900);
    write_u32(&mut lap_eleven, 23, 44);
    adapter.decode(&lap_eleven, at, &mut output)?;
    assert_eq!(output.events.len(), 1);
    assert_eq!(output.events[0].data["lap"], 10);
    Ok(())
}

#[test]
fn telemetry2_enforces_the_official_24_car_packet_shape() -> Result<(), Box<dyn Error>> {
    let mut adapter = F1_25Adapter::new()?;
    let mut output = AdapterOutput::default();
    let mut truncated = car_telemetry2_packet();
    truncated.pop();
    assert!(
        adapter
            .decode(&truncated, MonotonicTimestamp::default(), &mut output)
            .is_err()
    );

    let mut invalid_player = car_telemetry2_packet();
    invalid_player[27] = u8::try_from(F1_25_2026_CAR_COUNT)?;
    assert!(
        adapter
            .decode(&invalid_player, MonotonicTimestamp::default(), &mut output)
            .is_err()
    );
    Ok(())
}

#[test]
fn f1_24_and_original_f1_25_decode_their_extended_layouts() -> Result<(), Box<dyn Error>> {
    let at = MonotonicTimestamp::from_micros(2_000);
    let mut f1_24 = F1_24Adapter::new()?;
    let mut output = AdapterOutput::default();

    let mut session = legacy_packet(F1_24_PACKET_FORMAT, 24, 1, SESSION_PACKET_LEN);
    let p = HEADER_LEN;
    session[p + 3] = 50;
    write_u16(&mut session, p + 4, 5_000);
    session[p + 6] = 10;
    session[p + 7] = 4;
    write_u16(&mut session, p + 9, 900);
    session[p + 13] = 80;
    session[p + 124] = 1;
    f1_24.decode(&session, at, &mut output)?;
    assert_eq!(output.updates[0].session.total_laps, Some(50));
    assert_eq!(
        output.updates[0].session.safety_car_status,
        Some(SafetyCarStatus::Full)
    );

    output.clear();
    let mut lap = legacy_packet(F1_24_PACKET_FORMAT, 24, 2, LAP_DATA_PACKET_LEN);
    write_u32(&mut lap, p, 91_000);
    lap[p + 32] = 3;
    lap[p + 33] = 8;
    lap[p + 44] = 4;
    lap[p + 45] = 2;
    f1_24.decode(&lap, at, &mut output)?;
    assert_eq!(output.updates[0].lap.current, Some(8));
    assert_eq!(output.updates[0].lap.position, Some(3));

    output.clear();
    let mut status = legacy_packet(F1_24_PACKET_FORMAT, 24, 7, CAR_STATUS_PACKET_LEN);
    write_f32(&mut status, p + 5, 42.0);
    write_u16(&mut status, p + 17, 13_500);
    status[p + 25] = 18;
    f1_24.decode(&status, at, &mut output)?;
    assert_eq!(output.updates[0].vehicle.fuel_kg, Some(42.0));
    assert_eq!(output.updates[0].vehicle.rpm_max, Some(13_500));
    assert_eq!((status.len() - HEADER_LEN) / 22, CAR_STATUS_DATA_LEN);

    output.clear();
    f1_24.decode(
        &legacy_damage_packet(
            F1_24_PACKET_FORMAT,
            24,
            F1_24_CAR_DAMAGE_DATA_LEN,
            F1_24_CAR_DAMAGE_PACKET_LEN,
        ),
        at,
        &mut output,
    )?;
    let f1_24_front_left = output.updates[0].tyres.front_left.as_ref();
    assert_eq!(
        f1_24_front_left
            .and_then(|tyre| tyre.wear)
            .map(Normalized::get),
        Some(0.3)
    );
    assert_eq!(f1_24_front_left.and_then(|tyre| tyre.blister), None);

    output.clear();
    let mut f1_25 = F1_25Adapter::new()?;
    f1_25.decode(
        &legacy_damage_packet(
            F1_25_PACKET_FORMAT,
            25,
            F1_25_CAR_DAMAGE_DATA_LEN,
            F1_25_CAR_DAMAGE_PACKET_LEN,
        ),
        at,
        &mut output,
    )?;
    assert_eq!(
        output.updates[0]
            .tyres
            .front_left
            .as_ref()
            .and_then(|tyre| tyre.blister)
            .map(Normalized::get),
        Some(0.03)
    );
    Ok(())
}
