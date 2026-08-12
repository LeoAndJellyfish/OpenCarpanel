use opencarpanel_adapter_api::{
    AdapterDescriptor, AdapterError, AdapterId, AdapterOutput, CapabilitySet, GameAdapter,
};
use opencarpanel_telemetry_core::{MonotonicTimestamp, TelemetryField};

use crate::{
    CAR_TELEMETRY_PACKET_ID, DecodeError, F1_24_PROTOCOL, F1_25_PROTOCOL, F1Protocol,
    decode_header_and_layout, decode_player_sample, map_player_sample,
};

#[derive(Debug)]
struct F1Adapter {
    descriptor: AdapterDescriptor,
    protocol: F1Protocol,
}

impl F1Adapter {
    fn new(protocol: F1Protocol) -> Result<Self, AdapterError> {
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
            AdapterId::new(protocol.adapter_id)?,
            protocol.display_name,
            protocol.protocol_version,
            capabilities,
        );
        Ok(Self {
            descriptor,
            protocol,
        })
    }

    fn decode(
        &mut self,
        datagram: &[u8],
        received_at: MonotonicTimestamp,
        output: &mut AdapterOutput,
    ) -> Result<(), AdapterError> {
        let (header, payload, layout) = decode_header_and_layout(datagram, self.protocol)
            .map_err(|error| adapter_error(&error))?;
        if header.packet_id != CAR_TELEMETRY_PACKET_ID {
            return Ok(());
        }

        let sample = decode_player_sample(&header, payload, datagram.len(), layout)
            .map_err(|error| adapter_error(&error))?;
        let update = map_player_sample(&header, sample, received_at)
            .map_err(|error| adapter_error(&error))?;
        output.updates.push(update);
        Ok(())
    }
}

macro_rules! concrete_adapter {
    ($name:ident, $protocol:expr) => {
        #[doc = concat!("Built-in adapter for ", stringify!($name), ".")]
        #[derive(Debug)]
        pub struct $name {
            inner: F1Adapter,
        }

        impl $name {
            /// Builds the adapter and its stable capability descriptor.
            ///
            /// # Errors
            ///
            /// Returns [`AdapterError`] if compile-time metadata violates the
            /// shared adapter contract.
            pub fn new() -> Result<Self, AdapterError> {
                Ok(Self {
                    inner: F1Adapter::new($protocol)?,
                })
            }
        }

        impl GameAdapter for $name {
            fn descriptor(&self) -> &AdapterDescriptor {
                &self.inner.descriptor
            }

            fn decode(
                &mut self,
                datagram: &[u8],
                received_at: MonotonicTimestamp,
                output: &mut AdapterOutput,
            ) -> Result<(), AdapterError> {
                self.inner.decode(datagram, received_at, output)
            }
        }
    };
}

concrete_adapter!(F1_24Adapter, F1_24_PROTOCOL);
concrete_adapter!(F1_25Adapter, F1_25_PROTOCOL);

fn adapter_error(error: &DecodeError) -> AdapterError {
    match error {
        DecodeError::UnsupportedPacketFormat { expected, actual } => {
            AdapterError::unsupported_protocol(expected.to_string(), actual.to_string())
        }
        DecodeError::UnsupportedPacketFormats { expected, actual } => {
            AdapterError::unsupported_protocol(
                expected
                    .iter()
                    .map(u16::to_string)
                    .collect::<Vec<_>>()
                    .join(", "),
                actual.to_string(),
            )
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
