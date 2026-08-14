use opencarpanel_adapter_api::{
    AdapterDescriptor, AdapterError, AdapterId, AdapterOutput, CapabilitySet, GameAdapter,
};
use opencarpanel_telemetry_core::{DrsState, MonotonicTimestamp, TelemetryEvent, TelemetryField};
use serde_json::json;

use crate::mapping::{
    map_damage_sample, map_event_sample, map_lap_sample, map_player_sample, map_session_sample,
    map_status_sample, map_telemetry2_sample,
};
use crate::packets::{
    CAR_DAMAGE_PACKET_ID, CAR_STATUS_PACKET_ID, CAR_TELEMETRY_PACKET_ID, CAR_TELEMETRY2_PACKET_ID,
    EVENT_PACKET_ID, LAP_DATA_PACKET_ID, SESSION_PACKET_ID, decode_event, decode_player_car_damage,
    decode_player_car_status, decode_player_car_telemetry, decode_player_car_telemetry2,
    decode_player_lap, decode_session,
};
use crate::{DecodeError, F1_24_PROTOCOL, F1_25_2026_PACKET_FORMAT, F1_25_PROTOCOL, F1Protocol};

#[derive(Debug)]
struct F1Adapter {
    descriptor: AdapterDescriptor,
    protocol: F1Protocol,
    session_uid: Option<u64>,
    last_lap: Option<(u32, u16)>,
    drs_active: Option<(u32, bool)>,
    drs_allowed: Option<(u32, bool)>,
}

impl F1Adapter {
    fn new(protocol: F1Protocol) -> Result<Self, AdapterError> {
        let mut capabilities = CapabilitySet::from([
            TelemetryField::VehicleSpeed,
            TelemetryField::VehicleGear,
            TelemetryField::VehicleRpm,
            TelemetryField::VehicleRpmMax,
            TelemetryField::VehicleRevLights,
            TelemetryField::VehicleThrottle,
            TelemetryField::VehicleBrake,
            TelemetryField::VehicleDrs,
            TelemetryField::VehicleFuel,
            TelemetryField::VehiclePitLimiter,
            TelemetryField::LapCurrent,
            TelemetryField::LapPosition,
            TelemetryField::LapCurrentTime,
            TelemetryField::LapLastTime,
            TelemetryField::LapInvalid,
            TelemetryField::LapRaceState,
            TelemetryField::SessionTrack,
            TelemetryField::SessionRemainingTime,
            TelemetryField::SessionTotalLaps,
            TelemetryField::SessionRaceState,
            TelemetryField::Tyres,
            TelemetryField::Conditions,
            TelemetryField::Damage,
        ]);
        if protocol.packet_formats.contains(&F1_25_2026_PACKET_FORMAT) {
            capabilities.insert(TelemetryField::Aero);
        }
        let descriptor = AdapterDescriptor::new(
            AdapterId::new(protocol.adapter_id)?,
            protocol.display_name,
            protocol.protocol_version,
            capabilities,
        );
        Ok(Self {
            descriptor,
            protocol,
            session_uid: None,
            last_lap: None,
            drs_active: None,
            drs_allowed: None,
        })
    }

    fn decode(
        &mut self,
        datagram: &[u8],
        received_at: MonotonicTimestamp,
        output: &mut AdapterOutput,
    ) -> Result<(), AdapterError> {
        let (header, payload, layout) = crate::decode_header_and_layout(datagram, self.protocol)
            .map_err(|error| adapter_error(&error))?;
        self.ensure_session(header.session_uid);

        match header.packet_id {
            SESSION_PACKET_ID => {
                let sample = decode_session(&header, payload, datagram.len(), layout)
                    .map_err(|error| adapter_error(&error))?;
                output.updates.push(
                    map_session_sample(&header, sample, received_at)
                        .map_err(|error| adapter_error(&error))?,
                );
            }
            LAP_DATA_PACKET_ID => {
                let sample = decode_player_lap(&header, payload, datagram.len(), layout)
                    .map_err(|error| adapter_error(&error))?;
                let update = map_lap_sample(&header, sample, received_at)
                    .map_err(|error| adapter_error(&error))?;
                let current_lap = update.lap.current;
                if frame_can_replace(
                    self.last_lap.map(|(frame, _)| frame),
                    header.overall_frame_identifier,
                ) {
                    if let (Some((_, previous)), Some(current)) = (self.last_lap, current_lap)
                        && current > previous
                    {
                        output.events.push(TelemetryEvent {
                            name: "lap.completed".to_owned(),
                            occurred_at: received_at,
                            data: json!({
                                "lap": current.saturating_sub(1),
                                "lastTimeMs": update.lap.last_time_ms,
                            }),
                        });
                    }
                    self.last_lap =
                        current_lap.map(|current| (header.overall_frame_identifier, current));
                }
                output.updates.push(update);
            }
            EVENT_PACKET_ID => {
                let sample = decode_event(&header, payload, datagram.len(), layout)
                    .map_err(|error| adapter_error(&error))?;
                output.events.push(
                    map_event_sample(&header, sample, received_at)
                        .map_err(|error| adapter_error(&error))?,
                );
            }
            CAR_TELEMETRY_PACKET_ID => {
                let sample = decode_player_car_telemetry(&header, payload, datagram.len(), layout)
                    .map_err(|error| adapter_error(&error))?;
                update_framed(
                    &mut self.drs_active,
                    header.overall_frame_identifier,
                    sample.drs_active,
                );
                let drs = self.drs_state();
                output.updates.push(
                    map_player_sample(&header, sample, received_at, drs)
                        .map_err(|error| adapter_error(&error))?,
                );
            }
            CAR_STATUS_PACKET_ID => {
                let sample = decode_player_car_status(&header, payload, datagram.len(), layout)
                    .map_err(|error| adapter_error(&error))?;
                update_framed(
                    &mut self.drs_allowed,
                    header.overall_frame_identifier,
                    sample.drs_allowed,
                );
                let drs = self.drs_state();
                output.updates.push(
                    map_status_sample(&header, sample, received_at, drs)
                        .map_err(|error| adapter_error(&error))?,
                );
            }
            CAR_DAMAGE_PACKET_ID => {
                let sample = decode_player_car_damage(&header, payload, datagram.len(), layout)
                    .map_err(|error| adapter_error(&error))?;
                output.updates.push(
                    map_damage_sample(&header, sample, received_at)
                        .map_err(|error| adapter_error(&error))?,
                );
            }
            CAR_TELEMETRY2_PACKET_ID if layout.car_telemetry2.is_some() => {
                let sample = decode_player_car_telemetry2(&header, payload, datagram.len(), layout)
                    .map_err(|error| adapter_error(&error))?;
                output
                    .updates
                    .push(map_telemetry2_sample(&header, sample, received_at));
            }
            _ => {}
        }
        Ok(())
    }

    fn ensure_session(&mut self, session_uid: u64) {
        if self.session_uid != Some(session_uid) {
            self.session_uid = Some(session_uid);
            self.last_lap = None;
            self.drs_active = None;
            self.drs_allowed = None;
        }
    }

    fn drs_state(&self) -> DrsState {
        if self.drs_active.is_some_and(|(_, active)| active) {
            DrsState::Active
        } else {
            match self.drs_allowed.map(|(_, allowed)| allowed) {
                Some(true) => DrsState::Available,
                Some(false) => DrsState::Unavailable,
                None => DrsState::Unknown,
            }
        }
    }
}

fn update_framed<T>(target: &mut Option<(u32, T)>, frame: u32, value: T) {
    if frame_can_replace(target.as_ref().map(|(current, _)| *current), frame) {
        *target = Some((frame, value));
    }
}

fn frame_can_replace(current: Option<u32>, candidate: u32) -> bool {
    current.is_none_or(|current| {
        candidate == current || candidate.wrapping_sub(current) < (1_u32 << 31)
    })
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
