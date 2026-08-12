use opencarpanel_telemetry_core::{
    DrsState, Gear, MonotonicTimestamp, Normalized, TelemetryUpdate, VehicleUpdate,
};

use crate::{CarTelemetrySample, DecodeError, PacketHeader};

const KILOMETRES_PER_HOUR_PER_METRE_PER_SECOND: f32 = 3.6;

pub(crate) fn map_player_sample(
    header: &PacketHeader,
    sample: CarTelemetrySample,
    received_at: MonotonicTimestamp,
) -> Result<TelemetryUpdate, DecodeError> {
    let throttle =
        Normalized::new(sample.throttle).map_err(|_| DecodeError::InvalidNormalizedValue {
            field: "throttle",
            value: sample.throttle,
        })?;
    let brake = Normalized::new(sample.brake).map_err(|_| DecodeError::InvalidNormalizedValue {
        field: "brake",
        value: sample.brake,
    })?;
    let drs = match sample.drs {
        0 => DrsState::Unknown,
        1 => DrsState::Active,
        actual => {
            return Err(DecodeError::InvalidEnumValue {
                field: "drs",
                actual,
            });
        }
    };
    let rev_lights =
        Normalized::new(f32::from(sample.rev_lights_percent) / 100.0).map_err(|_| {
            DecodeError::InvalidNormalizedValue {
                field: "rev_lights",
                value: f32::from(sample.rev_lights_percent) / 100.0,
            }
        })?;

    Ok(TelemetryUpdate {
        received_at,
        session_id: Some(format!("{:016x}", header.session_uid)),
        frame_id: Some(header.overall_frame_identifier),
        vehicle: VehicleUpdate {
            speed_mps: Some(f32::from(sample.speed_kph) / KILOMETRES_PER_HOUR_PER_METRE_PER_SECOND),
            gear: Some(map_gear(sample.gear)),
            rpm: Some(sample.engine_rpm),
            rev_lights: Some(rev_lights),
            throttle: Some(throttle),
            brake: Some(brake),
            drs: Some(drs),
            ..VehicleUpdate::default()
        },
        ..TelemetryUpdate::default()
    })
}

fn map_gear(gear: i8) -> Gear {
    match gear {
        -1 => Gear::Reverse,
        0 => Gear::Neutral,
        1..=8 => u8::try_from(gear)
            .ok()
            .and_then(Gear::forward)
            .unwrap_or(Gear::Unknown),
        _ => Gear::Unknown,
    }
}
