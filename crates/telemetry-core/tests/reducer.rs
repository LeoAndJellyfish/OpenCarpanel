use opencarpanel_telemetry_core::{
    Gear, MonotonicTimestamp, Normalized, TelemetryReducer, TelemetryUpdate,
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
