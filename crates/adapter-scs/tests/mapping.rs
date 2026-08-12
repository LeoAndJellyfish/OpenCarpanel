use std::{error::Error, io};

use opencarpanel_adapter_api::{AdapterOutput, GameAdapter};
use opencarpanel_adapter_scs::{
    ATS_GAME_ID, AtsAdapter, BRIDGE_MAGIC, BRIDGE_PACKET_LEN, BRIDGE_PROTOCOL_VERSION,
    ETS2_GAME_ID, Ets2Adapter,
};
use opencarpanel_telemetry_core::{DrsState, Gear, MonotonicTimestamp, Normalized};

fn packet(game: u8, gear: i32) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(BRIDGE_PACKET_LEN);
    bytes.extend_from_slice(&BRIDGE_MAGIC);
    bytes.extend_from_slice(&[BRIDGE_PROTOCOL_VERSION, game, 0, 0]);
    bytes.extend_from_slice(&0xa1a2_a3a4_a5a6_a7a8_u64.to_le_bytes());
    bytes.extend_from_slice(&77_u32.to_le_bytes());
    bytes.extend_from_slice(&(-20.0_f32).to_le_bytes());
    bytes.extend_from_slice(&1_350.6_f32.to_le_bytes());
    bytes.extend_from_slice(&2_500.0_f32.to_le_bytes());
    bytes.extend_from_slice(&gear.to_le_bytes());
    bytes.extend_from_slice(&0.8_f32.to_le_bytes());
    bytes.extend_from_slice(&0.1_f32.to_le_bytes());
    bytes
}

fn assert_mapping<A: GameAdapter>(adapter: &mut A, game: u8) -> Result<(), Box<dyn Error>> {
    let mut output = AdapterOutput::with_capacity(1, 0);
    adapter.decode(
        &packet(game, 12),
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
            &packet(ATS_GAME_ID, 1),
            MonotonicTimestamp::default(),
            &mut output,
        )
        .is_err()
    );
    assert!(
        ats.decode(
            &packet(ETS2_GAME_ID, 1),
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
            &packet(ETS2_GAME_ID, wire),
            MonotonicTimestamp::default(),
            &mut output,
        )?;
        assert_eq!(output.updates[0].vehicle.gear, Some(expected));
    }
    Ok(())
}
