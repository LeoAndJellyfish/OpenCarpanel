use std::error::Error;

use opencarpanel_adapter_api::{
    AdapterDescriptor, AdapterError, AdapterId, AdapterOutput, CapabilitySet, GameAdapter,
};
use opencarpanel_telemetry_core::{MonotonicTimestamp, TelemetryField, TelemetryUpdate};

#[derive(Debug)]
struct FakeAdapter {
    descriptor: AdapterDescriptor,
}

impl FakeAdapter {
    fn new() -> Result<Self, AdapterError> {
        Ok(Self {
            descriptor: AdapterDescriptor::new(
                AdapterId::new("test-adapter")?,
                "Test Adapter",
                "1",
                CapabilitySet::from([TelemetryField::VehicleSpeed]),
            ),
        })
    }
}

impl GameAdapter for FakeAdapter {
    fn descriptor(&self) -> &AdapterDescriptor {
        &self.descriptor
    }

    fn decode(
        &mut self,
        datagram: &[u8],
        received_at: MonotonicTimestamp,
        output: &mut AdapterOutput,
    ) -> Result<(), AdapterError> {
        if datagram != [42] {
            return Err(AdapterError::malformed_packet("expected the fake packet"));
        }

        let mut update = TelemetryUpdate {
            received_at,
            ..TelemetryUpdate::default()
        };
        update.vehicle.speed_mps = Some(42.0);
        output.updates.push(update);
        Ok(())
    }
}

#[test]
fn a_game_adapter_decodes_into_reusable_output() -> Result<(), Box<dyn Error>> {
    let mut adapter = FakeAdapter::new()?;
    let mut output = AdapterOutput::with_capacity(4, 2);

    adapter.decode(&[42], MonotonicTimestamp::from_micros(1_234), &mut output)?;

    assert_eq!(adapter.descriptor().id.as_str(), "test-adapter");
    assert_eq!(output.updates.len(), 1);
    assert!(output.events.is_empty());
    assert_eq!(output.updates[0].received_at.as_micros(), 1_234);

    let update_capacity = output.updates.capacity();
    let event_capacity = output.events.capacity();
    output.clear();

    assert!(output.updates.is_empty());
    assert!(output.events.is_empty());
    assert_eq!(output.updates.capacity(), update_capacity);
    assert_eq!(output.events.capacity(), event_capacity);

    Ok(())
}

#[test]
fn adapter_ids_are_stable_lowercase_slugs() {
    assert!(AdapterId::new("f1-24").is_ok());

    for invalid in ["", "F1-24", "-f1", "f1-", "f1--24", "f1_24"] {
        assert!(AdapterId::new(invalid).is_err(), "accepted {invalid:?}");
    }
}

#[test]
fn capability_sets_are_ordered_and_deduplicated() {
    let capabilities = CapabilitySet::from([
        TelemetryField::VehicleRpm,
        TelemetryField::VehicleSpeed,
        TelemetryField::VehicleRpm,
    ]);

    assert_eq!(capabilities.len(), 2);
    assert!(capabilities.contains(TelemetryField::VehicleSpeed));
    assert_eq!(
        capabilities.iter().copied().collect::<Vec<_>>(),
        vec![TelemetryField::VehicleSpeed, TelemetryField::VehicleRpm]
    );
}

#[test]
fn malformed_packets_have_actionable_errors() {
    let error = AdapterError::malformed_packet("packet is shorter than its header");

    assert_eq!(
        error.to_string(),
        "malformed packet: packet is shorter than its header"
    );
}
