use opensimdash_adapter_api::{
    AdapterDescriptor, AdapterError, AdapterId, AdapterOutput, CapabilitySet, GameAdapter,
};
use opensimdash_telemetry_core::{
    DrsState, Gear, JobState, LightsState, MonotonicTimestamp, NavigationState, Normalized,
    TelemetryField, TelemetryUpdate, VehicleUpdate,
};

use crate::{ATS_ADAPTER_ID, BridgeGame, BridgePacket, DecodeError, ETS2_ADAPTER_ID};

#[derive(Debug)]
struct ScsAdapter {
    descriptor: AdapterDescriptor,
    game: BridgeGame,
}

impl ScsAdapter {
    fn new(
        game: BridgeGame,
        adapter_id: &'static str,
        display_name: &'static str,
    ) -> Result<Self, AdapterError> {
        let capabilities = CapabilitySet::from([
            TelemetryField::VehicleSpeed,
            TelemetryField::VehicleGear,
            TelemetryField::VehicleRpm,
            TelemetryField::VehicleRpmMax,
            TelemetryField::VehicleThrottle,
            TelemetryField::VehicleBrake,
            TelemetryField::VehicleDrs,
            TelemetryField::VehicleFuel,
            TelemetryField::Navigation,
            TelemetryField::Lights,
            TelemetryField::Job,
        ]);
        Ok(Self {
            descriptor: AdapterDescriptor::new(
                AdapterId::new(adapter_id)?,
                display_name,
                "scs-bridge/v1+v2 (SDK 1.14)",
                capabilities,
            ),
            game,
        })
    }

    fn decode(
        &mut self,
        datagram: &[u8],
        received_at: MonotonicTimestamp,
        output: &mut AdapterOutput,
    ) -> Result<(), AdapterError> {
        let packet = BridgePacket::decode(datagram).map_err(|error| adapter_error(&error))?;
        if packet.game != self.game {
            return Err(AdapterError::unsupported_protocol(
                format!("SCS game {}", self.game.wire_id()),
                format!("SCS game {}", packet.game.wire_id()),
            ));
        }
        output.updates.push(map_packet(packet, received_at)?);
        Ok(())
    }
}

macro_rules! concrete_adapter {
    ($name:ident, $game:expr, $id:expr, $display_name:expr) => {
        #[doc = concat!("Built-in telemetry adapter for ", $display_name, ".")]
        #[derive(Debug)]
        pub struct $name {
            inner: ScsAdapter,
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
                    inner: ScsAdapter::new($game, $id, $display_name)?,
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

concrete_adapter!(
    Ets2Adapter,
    BridgeGame::Ets2,
    ETS2_ADAPTER_ID,
    "Euro Truck Simulator 2"
);
concrete_adapter!(
    AtsAdapter,
    BridgeGame::Ats,
    ATS_ADAPTER_ID,
    "American Truck Simulator"
);

fn map_packet(
    packet: BridgePacket,
    received_at: MonotonicTimestamp,
) -> Result<TelemetryUpdate, AdapterError> {
    let rpm = rounded_rpm(packet.rpm);
    let rpm_max = (packet.rpm_max > 0.0).then(|| rounded_rpm(packet.rpm_max));
    let throttle = Normalized::new(packet.throttle)
        .map_err(|error| AdapterError::malformed_packet(error.to_string()))?;
    let brake = Normalized::new(packet.brake)
        .map_err(|error| AdapterError::malformed_packet(error.to_string()))?;
    let lights = packet
        .lights
        .map_or_else(LightsState::default, |lights| LightsState {
            parking: Some(lights.parking),
            low_beam: Some(lights.low_beam),
            high_beam: Some(lights.high_beam),
            beacon: Some(lights.beacon),
            brake: Some(lights.brake),
            reverse: Some(lights.reverse),
            left_indicator: Some(lights.left_indicator),
            right_indicator: Some(lights.right_indicator),
            hazard: Some(lights.hazard),
        });
    let job = packet.job.map_or_else(JobState::default, |job| JobState {
        active: Some(job.active),
        cargo: job.cargo,
        cargo_mass_kg: job.active.then_some(job.cargo_mass_kg),
        source_city: job.source_city,
        destination_city: job.destination_city,
        income: job.active.then_some(job.income),
        delivery_time: job.active.then_some(job.delivery_time),
        planned_distance_km: job.active.then_some(job.planned_distance_km),
        cargo_loaded: job.active.then_some(job.cargo_loaded),
        special: job.active.then_some(job.special),
    });

    Ok(TelemetryUpdate {
        received_at,
        session_id: Some(format!("{:016x}", packet.session_nonce)),
        frame_id: Some(packet.frame_sequence),
        vehicle: VehicleUpdate {
            speed_mps: Some(packet.speed_mps.abs()),
            gear: Some(map_gear(packet.displayed_gear)),
            rpm: Some(rpm),
            rpm_max,
            throttle: Some(throttle),
            brake: Some(brake),
            drs: Some(DrsState::Unavailable),
            fuel_liters: packet.fuel_liters,
            fuel_capacity_liters: packet.fuel_capacity_liters,
            fuel_range_km: packet.fuel_range_km,
            fuel_warning: packet.fuel_warning,
            ..VehicleUpdate::default()
        },
        navigation: NavigationState {
            distance_m: packet.navigation_distance_m,
            time_s: packet.navigation_time_s,
            // The SCS traffic subsystem uses non-positive values for special
            // states such as no limit, wrong-way travel, or fast travel.
            speed_limit_mps: packet
                .navigation_speed_limit_mps
                .filter(|speed_limit| *speed_limit > 0.0),
        },
        lights,
        job,
        ..TelemetryUpdate::default()
    })
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn rounded_rpm(value: f32) -> u16 {
    // `BridgePacket::decode` rejects non-finite, negative, and >u16::MAX RPM
    // values before mapping reaches this conversion.
    value.round() as u16
}

fn map_gear(value: i32) -> Gear {
    match value {
        i32::MIN..=-1 => Gear::Reverse,
        0 => Gear::Neutral,
        1..=255 => u8::try_from(value)
            .ok()
            .and_then(Gear::forward)
            .unwrap_or(Gear::Unknown),
        _ => Gear::Unknown,
    }
}

fn adapter_error(error: &DecodeError) -> AdapterError {
    match error {
        DecodeError::UnsupportedMagic { actual } => AdapterError::unsupported_protocol(
            String::from_utf8_lossy(&crate::BRIDGE_MAGIC),
            format!("{actual:02x?}"),
        ),
        DecodeError::UnsupportedVersion { expected, actual }
        | DecodeError::UnsupportedGame { expected, actual } => {
            AdapterError::unsupported_protocol(expected.to_string(), actual.to_string())
        }
        _ => AdapterError::malformed_packet(error.to_string()),
    }
}
