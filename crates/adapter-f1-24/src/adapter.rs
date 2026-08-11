use opencarpanel_adapter_api::{
    AdapterDescriptor, AdapterError, AdapterId, AdapterOutput, CapabilitySet, GameAdapter,
};
use opencarpanel_telemetry_core::{MonotonicTimestamp, TelemetryField, TelemetryUpdate};

use crate::{
    ADAPTER_ID, CAR_TELEMETRY_PACKET_ID, DecodeError, PacketHeader, decode_player_sample,
    map_player_sample,
};

/// Built-in adapter for the F1 24 v27.2x UDP protocol.
#[derive(Debug)]
pub struct F1_24Adapter {
    descriptor: AdapterDescriptor,
}

impl F1_24Adapter {
    /// Builds the adapter and its stable capability descriptor.
    ///
    /// # Errors
    ///
    /// Returns [`AdapterError`] if the compile-time adapter id violates the
    /// shared adapter-id contract.
    pub fn new() -> Result<Self, AdapterError> {
        let capabilities = CapabilitySet::from([
            TelemetryField::VehicleSpeed,
            TelemetryField::VehicleGear,
            TelemetryField::VehicleRpm,
            TelemetryField::VehicleRevLights,
            TelemetryField::VehicleThrottle,
            TelemetryField::VehicleBrake,
            TelemetryField::VehicleDrs,
        ]);
        let descriptor = AdapterDescriptor::new(
            AdapterId::new(ADAPTER_ID)?,
            "EA Sports F1 24",
            "v27.2x",
            capabilities,
        );
        Ok(Self { descriptor })
    }
}

impl GameAdapter for F1_24Adapter {
    fn descriptor(&self) -> &AdapterDescriptor {
        &self.descriptor
    }

    fn decode(
        &mut self,
        datagram: &[u8],
        received_at: MonotonicTimestamp,
        output: &mut AdapterOutput,
    ) -> Result<(), AdapterError> {
        let (header, payload) =
            PacketHeader::decode(datagram).map_err(|error| adapter_error(&error))?;
        if header.packet_id != CAR_TELEMETRY_PACKET_ID {
            return Ok(());
        }

        let sample = decode_player_sample(&header, payload, datagram.len())
            .map_err(|error| adapter_error(&error))?;
        let update = map_player_sample(&header, sample, received_at)
            .map_err(|error| adapter_error(&error))?;
        output.updates.push(update);
        Ok(())
    }
}

/// Decodes one complete Car Telemetry datagram into a canonical player update.
///
/// # Errors
///
/// Returns [`DecodeError`] for a malformed header, packet id, packet version,
/// length, player index, ratio, or enum value.
pub fn decode_player_car_telemetry(
    datagram: &[u8],
    received_at: MonotonicTimestamp,
) -> Result<TelemetryUpdate, DecodeError> {
    let (header, payload) = PacketHeader::decode(datagram)?;
    let sample = decode_player_sample(&header, payload, datagram.len())?;
    map_player_sample(&header, sample, received_at)
}

fn adapter_error(error: &DecodeError) -> AdapterError {
    match error {
        DecodeError::UnsupportedPacketFormat { expected, actual } => {
            AdapterError::unsupported_protocol(expected.to_string(), actual.to_string())
        }
        DecodeError::UnsupportedPacketVersion {
            packet_id,
            expected,
            actual,
        } => AdapterError::unsupported_protocol(
            format!("packet {packet_id} version {expected}"),
            format!("packet {packet_id} version {actual}"),
        ),
        _ => AdapterError::malformed_packet(error.to_string()),
    }
}
