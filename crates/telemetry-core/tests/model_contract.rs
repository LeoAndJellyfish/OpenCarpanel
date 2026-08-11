use std::error::Error;

use opencarpanel_telemetry_core::{
    Gear, Normalized, TELEMETRY_SCHEMA_VERSION, TelemetryField, TelemetrySnapshot, TelemetryUpdate,
};
use schemars::schema_for;

#[test]
fn absent_game_data_stays_absent() -> Result<(), Box<dyn Error>> {
    let snapshot = TelemetrySnapshot::default();

    assert_eq!(snapshot.meta.schema_version, TELEMETRY_SCHEMA_VERSION);
    assert_eq!(snapshot.vehicle.speed_mps, None);
    assert_eq!(snapshot.vehicle.gear, Gear::Unknown);
    assert!(snapshot.extensions.is_empty());

    let encoded = serde_json::to_value(snapshot)?;
    assert!(encoded["vehicle"].get("speedMps").is_none());
    assert!(encoded.get("extensions").is_none());

    Ok(())
}

#[test]
fn normalized_inputs_enforce_the_closed_unit_interval() -> Result<(), Box<dyn Error>> {
    for value in [0.0, 0.75, 1.0] {
        assert!((Normalized::new(value)?.get() - value).abs() <= f32::EPSILON);
    }

    for value in [-0.01, 1.01, f32::NAN, f32::INFINITY] {
        assert!(Normalized::new(value).is_err());
    }

    assert!(serde_json::from_str::<Normalized>("1.01").is_err());

    Ok(())
}

#[test]
fn normalized_schema_documents_its_range() -> Result<(), Box<dyn Error>> {
    let schema = serde_json::to_value(schema_for!(Normalized))?;

    assert_eq!(schema["minimum"], 0.0);
    assert_eq!(schema["maximum"], 1.0);

    Ok(())
}

#[test]
fn public_field_paths_match_the_wire_model() {
    assert_eq!(TelemetryField::VehicleSpeed.as_path(), "vehicle.speedMps");
    assert_eq!(TelemetryField::VehicleRpm.as_path(), "vehicle.rpm");
}

#[test]
fn an_empty_update_contains_no_accidental_values() {
    let update = TelemetryUpdate::default();

    assert_eq!(update.vehicle.speed_mps, None);
    assert_eq!(update.vehicle.gear, None);
    assert!(update.extensions.is_empty());
}
