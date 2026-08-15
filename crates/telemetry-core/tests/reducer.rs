use opensimdash_telemetry_core::{
    Gear, JobState, MonotonicTimestamp, Normalized, TelemetryReducer, TelemetryUpdate,
    TyreCornerState,
};

fn vehicle_update(session: &str, frame: u32) -> TelemetryUpdate {
    TelemetryUpdate {
        received_at: MonotonicTimestamp::from_micros(u64::from(frame) * 1_000),
        session_id: Some(session.into()),
        frame_id: Some(frame),
        ..TelemetryUpdate::default()
    }
}

#[test]
fn partial_updates_merge_into_one_snapshot() {
    let mut reducer = TelemetryReducer::default();
    let mut speed = vehicle_update("session-a", 10);
    speed.vehicle.speed_mps = Some(50.0);
    reducer.apply(speed);

    let mut rpm = vehicle_update("session-a", 11);
    rpm.vehicle.rpm = Some(11_000);
    rpm.vehicle.rev_lights = Normalized::new(0.92).ok();
    rpm.vehicle.gear = Gear::forward(7);
    reducer.apply(rpm);

    let snapshot = reducer.snapshot();
    assert_eq!(snapshot.vehicle.speed_mps, Some(50.0));
    assert_eq!(snapshot.vehicle.rpm, Some(11_000));
    assert_eq!(snapshot.vehicle.rev_lights.map(Normalized::get), Some(0.92));
    assert_eq!(
        snapshot.vehicle.gear,
        Gear::forward(7).unwrap_or(Gear::Unknown)
    );
    assert_eq!(snapshot.meta.sequence, 2);
}

#[test]
fn new_session_clears_stale_game_state() {
    let mut reducer = TelemetryReducer::default();
    let mut old = vehicle_update("session-a", 30);
    old.vehicle.speed_mps = Some(60.0);
    reducer.apply(old);

    let mut new = vehicle_update("session-b", 1);
    new.vehicle.rpm = Some(9_000);
    reducer.apply(new);

    let snapshot = reducer.snapshot();
    assert_eq!(snapshot.meta.session_id.as_deref(), Some("session-b"));
    assert_eq!(snapshot.vehicle.speed_mps, None);
    assert_eq!(snapshot.vehicle.rpm, Some(9_000));
}

#[test]
fn older_frame_cannot_overwrite_a_newer_field() {
    let mut reducer = TelemetryReducer::default();
    let mut newer = vehicle_update("session-a", 10);
    newer.vehicle.speed_mps = Some(50.0);
    reducer.apply(newer);

    let mut older = vehicle_update("session-a", 9);
    older.received_at = MonotonicTimestamp::from_micros(11_000);
    older.vehicle.speed_mps = Some(40.0);
    older.vehicle.rpm = Some(11_000);
    reducer.apply(older);

    let snapshot = reducer.snapshot();
    assert_eq!(snapshot.vehicle.speed_mps, Some(50.0));
    assert_eq!(snapshot.vehicle.rpm, Some(11_000));
    assert_eq!(
        snapshot.meta.captured_at,
        Some(MonotonicTimestamp::from_micros(11_000))
    );
}

#[test]
fn tyre_packets_merge_independent_temperature_and_damage_fields() {
    let mut reducer = TelemetryReducer::default();
    let mut temperatures = vehicle_update("session-a", 20);
    temperatures.tyres.front_left = Some(TyreCornerState {
        surface_temperature_c: Some(92.0),
        inner_temperature_c: Some(88.0),
        pressure_pa: Some(145_000.0),
        ..TyreCornerState::default()
    });
    reducer.apply(temperatures);

    let mut damage = vehicle_update("session-a", 21);
    damage.tyres.front_left = Some(TyreCornerState {
        wear: Normalized::new(0.24).ok(),
        damage: Normalized::new(0.08).ok(),
        brake_damage: Normalized::new(0.03).ok(),
        ..TyreCornerState::default()
    });
    reducer.apply(damage);

    let tyre = reducer.snapshot().tyres.front_left.as_ref();
    assert_eq!(
        tyre.and_then(|value| value.surface_temperature_c),
        Some(92.0)
    );
    assert_eq!(tyre.and_then(|value| value.inner_temperature_c), Some(88.0));
    assert_eq!(tyre.and_then(|value| value.pressure_pa), Some(145_000.0));
    assert_eq!(
        tyre.and_then(|value| value.wear).map(Normalized::get),
        Some(0.24)
    );
    assert_eq!(
        tyre.and_then(|value| value.damage).map(Normalized::get),
        Some(0.08)
    );
    assert_eq!(
        tyre.and_then(|value| value.brake_damage)
            .map(Normalized::get),
        Some(0.03)
    );
}

#[test]
fn completed_job_clears_stale_delivery_details() {
    let mut reducer = TelemetryReducer::default();
    let mut active = vehicle_update("session-a", 30);
    active.job = JobState {
        active: Some(true),
        cargo: Some("Medical Vaccines".into()),
        cargo_mass_kg: Some(22_000.0),
        source_city: Some("Hamburg".into()),
        destination_city: Some("Oslo".into()),
        income: Some(85_000),
        delivery_time: Some(11_000),
        planned_distance_km: Some(650),
        cargo_loaded: Some(true),
        special: Some(true),
    };
    reducer.apply(active);

    let mut completed = vehicle_update("session-a", 31);
    completed.job.active = Some(false);
    reducer.apply(completed);

    assert_eq!(
        reducer.snapshot().job,
        JobState {
            active: Some(false),
            ..JobState::default()
        }
    );

    let mut late = vehicle_update("session-a", 30);
    late.received_at = MonotonicTimestamp::from_micros(32_000);
    late.job = JobState {
        active: Some(true),
        cargo: Some("Stale cargo".into()),
        destination_city: Some("Stale city".into()),
        ..JobState::default()
    };
    reducer.apply(late);
    assert_eq!(
        reducer.snapshot().job,
        JobState {
            active: Some(false),
            ..JobState::default()
        }
    );
}
