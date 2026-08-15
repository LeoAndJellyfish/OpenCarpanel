use std::{error::Error, io};

use opensimdash_adapter_api::{AdapterOutput, GameAdapter};
use opensimdash_adapter_scs::{
    ATS_GAME_ID, AtsAdapter, BRIDGE_JOB_TEXT_LEN, BRIDGE_MAGIC, BRIDGE_PACKET_LEN,
    BRIDGE_PROTOCOL_V1, BRIDGE_PROTOCOL_VERSION, BRIDGE_V1_PACKET_LEN, ETS2_GAME_ID, Ets2Adapter,
};
use opensimdash_telemetry_core::{DrsState, Gear, MonotonicTimestamp, Normalized};

fn packet_v1(game: u8, gear: i32) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(BRIDGE_V1_PACKET_LEN);
    bytes.extend_from_slice(&BRIDGE_MAGIC);
    bytes.extend_from_slice(&[BRIDGE_PROTOCOL_V1, game, 0, 0]);
    bytes.extend_from_slice(&0xa1a2_a3a4_a5a6_a7a8_u64.to_le_bytes());
    bytes.extend_from_slice(&77_u32.to_le_bytes());
    bytes.extend_from_slice(&(-20.0_f32).to_le_bytes());
    bytes.extend_from_slice(&1_350.6_f32.to_le_bytes());
    bytes.extend_from_slice(&2_500.0_f32.to_le_bytes());
    bytes.extend_from_slice(&gear.to_le_bytes());
    bytes.extend_from_slice(&0.8_f32.to_le_bytes());
    bytes.extend_from_slice(&0.1_f32.to_le_bytes());
    assert_eq!(bytes.len(), BRIDGE_V1_PACKET_LEN);
    bytes
}

fn extend_text(bytes: &mut Vec<u8>, value: &str) {
    bytes.extend_from_slice(value.as_bytes());
    bytes.resize(bytes.len() + BRIDGE_JOB_TEXT_LEN - value.len(), 0);
}

fn packet_v2(game: u8, gear: i32) -> Vec<u8> {
    let mut bytes = packet_v1(game, gear);
    bytes[4] = BRIDGE_PROTOCOL_VERSION;
    bytes.extend_from_slice(&12_500.0_f32.to_le_bytes());
    bytes.extend_from_slice(&600.0_f32.to_le_bytes());
    bytes.extend_from_slice(&25.0_f32.to_le_bytes());
    bytes.extend_from_slice(&275.0_f32.to_le_bytes());
    bytes.extend_from_slice(&400.0_f32.to_le_bytes());
    bytes.extend_from_slice(&900.0_f32.to_le_bytes());
    bytes.extend_from_slice(&0x01d3_u16.to_le_bytes());
    bytes.extend_from_slice(&0x000f_u16.to_le_bytes());
    bytes.extend_from_slice(&11_000_u32.to_le_bytes());
    bytes.extend_from_slice(&650_u32.to_le_bytes());
    bytes.extend_from_slice(&85_000_u64.to_le_bytes());
    bytes.extend_from_slice(&22_000.0_f32.to_le_bytes());
    extend_text(&mut bytes, "Medical Vaccines");
    extend_text(&mut bytes, "Hamburg");
    extend_text(&mut bytes, "Oslo");
    assert_eq!(bytes.len(), BRIDGE_PACKET_LEN);
    bytes
}

fn assert_mapping<A: GameAdapter>(adapter: &mut A, game: u8) -> Result<(), Box<dyn Error>> {
    let mut output = AdapterOutput::with_capacity(1, 0);
    adapter.decode(
        &packet_v1(game, 12),
        MonotonicTimestamp::from_micros(900),
        &mut output,
    )?;
    let update = output
        .updates
        .first()
        .ok_or_else(|| io::Error::other("adapter emitted no update"))?;

    assert_eq!(update.received_at, MonotonicTimestamp::from_micros(900));
    assert_eq!(update.session_id.as_deref(), Some("a1a2a3a4a5a6a7a8"));
    assert_eq!(update.frame_id, Some(77));
    assert_eq!(update.vehicle.speed_mps, Some(20.0));
    assert_eq!(update.vehicle.rpm, Some(1_351));
    assert_eq!(update.vehicle.rpm_max, Some(2_500));
    assert_eq!(update.vehicle.gear, Gear::forward(12));
    assert_eq!(update.vehicle.throttle.map(Normalized::get), Some(0.8));
    assert_eq!(update.vehicle.brake.map(Normalized::get), Some(0.1));
    assert_eq!(update.vehicle.drs, Some(DrsState::Unavailable));
    Ok(())
}

#[test]
fn v2_maps_navigation_fuel_lights_and_delivery_job() -> Result<(), Box<dyn Error>> {
    let mut adapter = Ets2Adapter::new()?;
    let mut output = AdapterOutput::default();
    adapter.decode(
        &packet_v2(ETS2_GAME_ID, 7),
        MonotonicTimestamp::from_micros(901),
        &mut output,
    )?;
    let update = &output.updates[0];

    assert_eq!(update.vehicle.fuel_liters, Some(275.0));
    assert_eq!(update.vehicle.fuel_capacity_liters, Some(400.0));
    assert_eq!(update.vehicle.fuel_range_km, Some(900.0));
    assert_eq!(update.vehicle.fuel_warning, Some(true));
    assert_eq!(update.navigation.distance_m, Some(12_500.0));
    assert_eq!(update.navigation.time_s, Some(600.0));
    assert_eq!(update.navigation.speed_limit_mps, Some(25.0));
    assert_eq!(update.lights.parking, Some(true));
    assert_eq!(update.lights.low_beam, Some(true));
    assert_eq!(update.lights.high_beam, Some(false));
    assert_eq!(update.lights.brake, Some(true));
    assert_eq!(update.lights.left_indicator, Some(true));
    assert_eq!(update.lights.right_indicator, Some(true));
    assert_eq!(update.lights.hazard, Some(true));
    assert_eq!(update.job.active, Some(true));
    assert_eq!(update.job.cargo.as_deref(), Some("Medical Vaccines"));
    assert_eq!(update.job.cargo_mass_kg, Some(22_000.0));
    assert_eq!(update.job.source_city.as_deref(), Some("Hamburg"));
    assert_eq!(update.job.destination_city.as_deref(), Some("Oslo"));
    assert_eq!(update.job.income, Some(85_000));
    assert_eq!(update.job.delivery_time, Some(11_000));
    assert_eq!(update.job.planned_distance_km, Some(650));
    assert_eq!(update.job.cargo_loaded, Some(true));
    assert_eq!(update.job.special, Some(true));
    Ok(())
}

#[test]
fn v2_hides_non_positive_scs_speed_limit_special_states() -> Result<(), Box<dyn Error>> {
    let mut adapter = Ets2Adapter::new()?;
    for speed_limit in [0.0_f32, -1.0] {
        let mut bytes = packet_v2(ETS2_GAME_ID, 7);
        bytes[52..56].copy_from_slice(&speed_limit.to_le_bytes());
        let mut output = AdapterOutput::default();
        adapter.decode(&bytes, MonotonicTimestamp::default(), &mut output)?;
        assert_eq!(output.updates[0].navigation.speed_limit_mps, None);
    }
    Ok(())
}

#[test]
fn ets2_and_ats_map_the_common_truck_dashboard_fields() -> Result<(), Box<dyn Error>> {
    let mut ets2 = Ets2Adapter::new()?;
    let mut ats = AtsAdapter::new()?;
    assert_eq!(ets2.descriptor().id.as_str(), "ets2");
    assert_eq!(ats.descriptor().id.as_str(), "ats");
    assert_mapping(&mut ets2, ETS2_GAME_ID)?;
    assert_mapping(&mut ats, ATS_GAME_ID)?;
    Ok(())
}

#[test]
fn concrete_adapters_reject_the_other_truck_game() -> Result<(), Box<dyn Error>> {
    let mut ets2 = Ets2Adapter::new()?;
    let mut ats = AtsAdapter::new()?;
    let mut output = AdapterOutput::default();
    assert!(
        ets2.decode(
            &packet_v1(ATS_GAME_ID, 1),
            MonotonicTimestamp::default(),
            &mut output,
        )
        .is_err()
    );
    assert!(
        ats.decode(
            &packet_v1(ETS2_GAME_ID, 1),
            MonotonicTimestamp::default(),
            &mut output,
        )
        .is_err()
    );
    Ok(())
}

#[test]
fn truck_gears_preserve_reverse_neutral_forward_and_unknown() -> Result<(), Box<dyn Error>> {
    let mut adapter = Ets2Adapter::new()?;
    for (wire, expected) in [
        (-2, Gear::Reverse),
        (0, Gear::Neutral),
        (18, Gear::forward(18).unwrap_or(Gear::Unknown)),
        (256, Gear::Unknown),
    ] {
        let mut output = AdapterOutput::default();
        adapter.decode(
            &packet_v1(ETS2_GAME_ID, wire),
            MonotonicTimestamp::default(),
            &mut output,
        )?;
        assert_eq!(output.updates[0].vehicle.gear, Some(expected));
    }
    Ok(())
}
