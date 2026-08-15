use opensimdash_telemetry_core::{
    ActiveAeroMode, AeroState, ConditionsState, DamageState, DriverStatus, DrsState, Gear,
    LapUpdate, MonotonicTimestamp, Normalized, PitStatus, RaceFlag, ResultStatus, SafetyCarStatus,
    SessionUpdate, TelemetryEvent, TelemetryUpdate, TyreCornerState, TyreUpdate, VehicleUpdate,
    WeatherCondition,
};
use serde_json::{Value, json};

use crate::packets::{
    CarDamageSample, CarStatusSample, CarTelemetry2Sample, CarTelemetrySample, EventSample,
    LapSample, SessionSample,
};
use crate::{Cursor, DecodeError, PacketHeader};

const KILOMETRES_PER_HOUR_PER_METRE_PER_SECOND: f32 = 3.6;
const PASCALS_PER_PSI: f32 = 6_894.757;

pub(crate) fn map_player_sample(
    header: &PacketHeader,
    sample: CarTelemetrySample,
    received_at: MonotonicTimestamp,
    drs: DrsState,
) -> Result<TelemetryUpdate, DecodeError> {
    let throttle = normalized("throttle", sample.throttle)?;
    let brake = normalized("brake", sample.brake)?;
    let rev_lights = normalized("rev_lights", f32::from(sample.rev_lights_percent) / 100.0)?;

    Ok(TelemetryUpdate {
        received_at,
        session_id: Some(session_id(header)),
        frame_id: Some(frame_id(header)),
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
        tyres: map_telemetry_tyres(sample),
        ..TelemetryUpdate::default()
    })
}

pub(crate) fn map_session_sample(
    header: &PacketHeader,
    sample: SessionSample,
    received_at: MonotonicTimestamp,
) -> Result<TelemetryUpdate, DecodeError> {
    let weather = match sample.weather {
        0 => WeatherCondition::Clear,
        1 => WeatherCondition::LightCloud,
        2 => WeatherCondition::Overcast,
        3 => WeatherCondition::LightRain,
        4 => WeatherCondition::HeavyRain,
        5 => WeatherCondition::Storm,
        actual => {
            return Err(DecodeError::InvalidEnumValue {
                field: "weather",
                actual,
            });
        }
    };
    let safety_car_status = match sample.safety_car_status {
        0 => SafetyCarStatus::None,
        1 => SafetyCarStatus::Full,
        2 => SafetyCarStatus::Virtual,
        3 => SafetyCarStatus::FormationLap,
        actual => {
            return Err(DecodeError::InvalidEnumValue {
                field: "safety_car_status",
                actual,
            });
        }
    };

    Ok(TelemetryUpdate {
        received_at,
        session_id: Some(session_id(header)),
        frame_id: Some(frame_id(header)),
        session: SessionUpdate {
            track_id: (sample.track_id >= 0).then(|| sample.track_id.to_string()),
            remaining_time_ms: Some(u64::from(sample.session_time_left_s) * 1_000),
            total_laps: Some(u16::from(sample.total_laps)),
            session_type: Some(session_type(sample.session_type).to_owned()),
            track_length_m: Some(u32::from(sample.track_length_m)),
            pit_speed_limit_mps: Some(
                f32::from(sample.pit_speed_limit_kph) / KILOMETRES_PER_HOUR_PER_METRE_PER_SECOND,
            ),
            safety_car_status: Some(safety_car_status),
            ..SessionUpdate::default()
        },
        conditions: ConditionsState {
            weather: Some(weather),
            track_temperature_c: Some(f32::from(sample.track_temperature_c)),
            air_temperature_c: Some(f32::from(sample.air_temperature_c)),
        },
        ..TelemetryUpdate::default()
    })
}

pub(crate) fn map_lap_sample(
    header: &PacketHeader,
    sample: LapSample,
    received_at: MonotonicTimestamp,
) -> Result<TelemetryUpdate, DecodeError> {
    let pit_status = map_pit_status(sample.pit_status)?;
    let driver_status = map_driver_status(sample.driver_status)?;
    let result_status = map_result_status(sample.result_status)?;
    let sector = match sample.sector {
        0..=2 => sample.sector + 1,
        actual => {
            return Err(DecodeError::InvalidEnumValue {
                field: "sector",
                actual,
            });
        }
    };
    let invalid = bool_value("current_lap_invalid", sample.current_lap_invalid)?;
    let pit_lane_timer_active = bool_value("pit_lane_timer_active", sample.pit_lane_timer_active)?;
    let pit_stop_should_serve_penalty = bool_value(
        "pit_stop_should_serve_penalty",
        sample.pit_stop_should_serve_penalty,
    )?;

    Ok(TelemetryUpdate {
        received_at,
        session_id: Some(session_id(header)),
        frame_id: Some(frame_id(header)),
        lap: LapUpdate {
            current: Some(u16::from(sample.current_lap_number)),
            position: Some(sample.car_position),
            current_time_ms: Some(sample.current_lap_time_ms),
            last_time_ms: Some(sample.last_lap_time_ms),
            invalid: Some(invalid),
            sector: Some(sector),
            sector1_time_ms: Some(compound_time_ms(
                sample.sector1_time_minutes_part,
                sample.sector1_time_ms_part,
            )),
            sector2_time_ms: Some(compound_time_ms(
                sample.sector2_time_minutes_part,
                sample.sector2_time_ms_part,
            )),
            delta_to_car_in_front_ms: Some(compound_time_ms(
                sample.delta_to_car_in_front_minutes_part,
                sample.delta_to_car_in_front_ms_part,
            )),
            delta_to_race_leader_ms: Some(compound_time_ms(
                sample.delta_to_race_leader_minutes_part,
                sample.delta_to_race_leader_ms_part,
            )),
            distance_m: Some(sample.lap_distance_m),
            total_distance_m: Some(sample.total_distance_m),
            safety_car_delta_ms: Some(seconds_to_millis_i32(
                "safety_car_delta",
                sample.safety_car_delta_s,
            )?),
            pit_status: Some(pit_status),
            pit_stops: Some(sample.pit_stops),
            penalties_seconds: Some(sample.penalties_seconds),
            warnings: Some(sample.warnings),
            corner_cutting_warnings: Some(sample.corner_cutting_warnings),
            unserved_drive_through_penalties: Some(sample.unserved_drive_through_penalties),
            unserved_stop_go_penalties: Some(sample.unserved_stop_go_penalties),
            grid_position: Some(sample.grid_position),
            driver_status: Some(driver_status),
            result_status: Some(result_status),
            pit_lane_time_ms: pit_lane_timer_active.then_some(u32::from(sample.pit_lane_time_ms)),
            pit_stop_time_ms: Some(u32::from(sample.pit_stop_time_ms)),
            pit_stop_should_serve_penalty: Some(pit_stop_should_serve_penalty),
            ..LapUpdate::default()
        },
        ..TelemetryUpdate::default()
    })
}

fn map_pit_status(value: u8) -> Result<PitStatus, DecodeError> {
    match value {
        0 => Ok(PitStatus::None),
        1 => Ok(PitStatus::Pitting),
        2 => Ok(PitStatus::InPitArea),
        actual => Err(DecodeError::InvalidEnumValue {
            field: "pit_status",
            actual,
        }),
    }
}

fn map_driver_status(value: u8) -> Result<DriverStatus, DecodeError> {
    match value {
        0 => Ok(DriverStatus::InGarage),
        1 => Ok(DriverStatus::FlyingLap),
        2 => Ok(DriverStatus::InLap),
        3 => Ok(DriverStatus::OutLap),
        4 => Ok(DriverStatus::OnTrack),
        actual => Err(DecodeError::InvalidEnumValue {
            field: "driver_status",
            actual,
        }),
    }
}

fn map_result_status(value: u8) -> Result<ResultStatus, DecodeError> {
    match value {
        0 => Ok(ResultStatus::Invalid),
        1 => Ok(ResultStatus::Inactive),
        2 => Ok(ResultStatus::Active),
        3 => Ok(ResultStatus::Finished),
        4 => Ok(ResultStatus::DidNotFinish),
        5 => Ok(ResultStatus::Disqualified),
        6 => Ok(ResultStatus::NotClassified),
        7 => Ok(ResultStatus::Retired),
        actual => Err(DecodeError::InvalidEnumValue {
            field: "result_status",
            actual,
        }),
    }
}

pub(crate) fn map_status_sample(
    header: &PacketHeader,
    sample: CarStatusSample,
    received_at: MonotonicTimestamp,
    drs: DrsState,
) -> Result<TelemetryUpdate, DecodeError> {
    let race_flag = match sample.vehicle_fia_flag {
        -1 => None,
        0 => Some(RaceFlag::None),
        1 => Some(RaceFlag::Green),
        2 => Some(RaceFlag::Blue),
        3 => Some(RaceFlag::Yellow),
        4 => Some(RaceFlag::Red),
        actual => {
            return Err(DecodeError::InvalidEnumValue {
                field: "vehicle_fia_flags",
                actual: actual.to_le_bytes()[0],
            });
        }
    };
    Ok(TelemetryUpdate {
        received_at,
        session_id: Some(session_id(header)),
        frame_id: Some(frame_id(header)),
        vehicle: VehicleUpdate {
            rpm_max: Some(sample.rpm_max),
            drs: Some(drs),
            fuel_kg: Some(sample.fuel_kg),
            fuel_capacity_kg: Some(sample.fuel_capacity_kg),
            fuel_remaining_laps: Some(sample.fuel_remaining_laps),
            pit_limiter: Some(sample.pit_limiter_active),
            ers_energy_j: Some(sample.ers_store_energy_j),
            ..VehicleUpdate::default()
        },
        session: SessionUpdate {
            race_flag,
            ..SessionUpdate::default()
        },
        tyres: TyreUpdate {
            actual_compound: Some(sample.actual_tyre_compound),
            visual_compound: Some(sample.visual_tyre_compound),
            age_laps: Some(sample.tyre_age_laps),
            ..TyreUpdate::default()
        },
        ..TelemetryUpdate::default()
    })
}

pub(crate) fn map_damage_sample(
    header: &PacketHeader,
    sample: CarDamageSample,
    received_at: MonotonicTimestamp,
) -> Result<TelemetryUpdate, DecodeError> {
    let corner = |index: usize| -> Result<TyreCornerState, DecodeError> {
        Ok(TyreCornerState {
            wear: Some(percent_f32("tyre_wear", sample.tyre_wear_percent[index])?),
            damage: Some(percent_u8(
                "tyre_damage",
                sample.tyre_damage_percent[index],
            )?),
            brake_damage: Some(percent_u8(
                "brake_damage",
                sample.brake_damage_percent[index],
            )?),
            blister: sample
                .tyre_blister_percent
                .map(|values| percent_u8("tyre_blister", values[index]))
                .transpose()?,
            ..TyreCornerState::default()
        })
    };
    Ok(TelemetryUpdate {
        received_at,
        session_id: Some(session_id(header)),
        frame_id: Some(frame_id(header)),
        tyres: TyreUpdate {
            rear_left: Some(corner(0)?),
            rear_right: Some(corner(1)?),
            front_left: Some(corner(2)?),
            front_right: Some(corner(3)?),
            ..TyreUpdate::default()
        },
        damage: DamageState {
            front_left_wing: Some(percent_u8(
                "front_left_wing",
                sample.front_left_wing_percent,
            )?),
            front_right_wing: Some(percent_u8(
                "front_right_wing",
                sample.front_right_wing_percent,
            )?),
            rear_wing: Some(percent_u8("rear_wing", sample.rear_wing_percent)?),
            floor: Some(percent_u8("floor", sample.floor_percent)?),
            diffuser: Some(percent_u8("diffuser", sample.diffuser_percent)?),
            sidepod: Some(percent_u8("sidepod", sample.sidepod_percent)?),
            gearbox: Some(percent_u8("gearbox", sample.gearbox_percent)?),
            engine: Some(percent_u8("engine", sample.engine_percent)?),
            engine_mguh_wear: Some(percent_u8(
                "engine_mguh_wear",
                sample.engine_mguh_wear_percent,
            )?),
            engine_es_wear: Some(percent_u8("engine_es_wear", sample.engine_es_wear_percent)?),
            engine_ce_wear: Some(percent_u8("engine_ce_wear", sample.engine_ce_wear_percent)?),
            engine_ice_wear: Some(percent_u8(
                "engine_ice_wear",
                sample.engine_ice_wear_percent,
            )?),
            engine_mguk_wear: Some(percent_u8(
                "engine_mguk_wear",
                sample.engine_mguk_wear_percent,
            )?),
            engine_tc_wear: Some(percent_u8("engine_tc_wear", sample.engine_tc_wear_percent)?),
            drs_fault: Some(sample.drs_fault),
            ers_fault: Some(sample.ers_fault),
            engine_blown: Some(sample.engine_blown),
            engine_seized: Some(sample.engine_seized),
        },
        ..TelemetryUpdate::default()
    })
}

pub(crate) fn map_telemetry2_sample(
    header: &PacketHeader,
    sample: CarTelemetry2Sample,
    received_at: MonotonicTimestamp,
) -> TelemetryUpdate {
    TelemetryUpdate {
        received_at,
        session_id: Some(session_id(header)),
        frame_id: Some(frame_id(header)),
        aero: AeroState {
            mode: Some(if sample.active_aero_mode == 0 {
                ActiveAeroMode::Corner
            } else {
                ActiveAeroMode::Straight
            }),
            available: Some(sample.active_aero_available),
            activation_distance_m: Some(u32::from(sample.active_aero_activation_distance_m)),
            overtake_available: Some(sample.overtake_available),
            overtake_active: Some(sample.overtake_active),
            overtake_activation_distance_m: Some(u32::from(sample.overtake_activation_distance_m)),
            regulations_2026: Some(sample.regulations_2026),
            driving_wrong_way: Some(sample.driving_wrong_way),
        },
        ..TelemetryUpdate::default()
    }
}

#[allow(clippy::too_many_lines)]
pub(crate) fn map_event_sample(
    header: &PacketHeader,
    sample: EventSample,
    occurred_at: MonotonicTimestamp,
) -> Result<TelemetryEvent, DecodeError> {
    let mut cursor = Cursor::new(&sample.details);
    let player = header.player_car_index;
    let (name, data) = match &sample.code {
        b"SSTA" => ("session.started", Value::Null),
        b"SEND" => ("session.ended", Value::Null),
        b"FTLP" => {
            let vehicle = cursor.read_u8()?;
            let lap_time = cursor.read_f32_le()?;
            finite("fastest_lap_time", lap_time)?;
            (
                "lap.fastest",
                json!({"vehicleIndex": vehicle, "isPlayer": vehicle == player, "lapTimeMs": seconds_to_millis_u32("fastest_lap_time", lap_time)?}),
            )
        }
        b"RTMT" => {
            let vehicle = cursor.read_u8()?;
            let reason = cursor.read_u8()?;
            (
                "driver.retired",
                json!({"vehicleIndex": vehicle, "isPlayer": vehicle == player, "reason": reason}),
            )
        }
        b"DRSE" => ("drs.enabled", Value::Null),
        b"DRSD" => ("drs.disabled", json!({"reason": cursor.read_u8()?})),
        b"TMPT" => {
            let vehicle = cursor.read_u8()?;
            (
                "teammate.pits",
                json!({"vehicleIndex": vehicle, "isPlayer": vehicle == player}),
            )
        }
        b"CHQF" => ("session.chequered_flag", Value::Null),
        b"RCWN" => {
            let vehicle = cursor.read_u8()?;
            (
                "race.winner",
                json!({"vehicleIndex": vehicle, "isPlayer": vehicle == player}),
            )
        }
        b"PENA" => {
            let penalty_type = cursor.read_u8()?;
            let infringement_type = cursor.read_u8()?;
            let vehicle = cursor.read_u8()?;
            let other_vehicle = cursor.read_u8()?;
            let time_seconds = cursor.read_u8()?;
            let lap_number = cursor.read_u8()?;
            let places_gained = cursor.read_u8()?;
            (
                "penalty.issued",
                json!({
                    "penaltyType": penalty_type,
                    "infringementType": infringement_type,
                    "vehicleIndex": vehicle,
                    "isPlayer": vehicle == player,
                    "otherVehicleIndex": other_vehicle,
                    "timeSeconds": time_seconds,
                    "lapNumber": lap_number,
                    "placesGained": places_gained
                }),
            )
        }
        b"SPTP" => {
            let vehicle = cursor.read_u8()?;
            let speed_kph = cursor.read_f32_le()?;
            finite("speed_trap_speed", speed_kph)?;
            let overall_fastest = bool_value("overall_fastest", cursor.read_u8()?)?;
            let driver_fastest = bool_value("driver_fastest", cursor.read_u8()?)?;
            let fastest_vehicle = cursor.read_u8()?;
            let fastest_speed_kph = cursor.read_f32_le()?;
            finite("fastest_speed", fastest_speed_kph)?;
            (
                "speed_trap.triggered",
                json!({
                    "vehicleIndex": vehicle,
                    "isPlayer": vehicle == player,
                    "speedMps": speed_kph / KILOMETRES_PER_HOUR_PER_METRE_PER_SECOND,
                    "overallFastest": overall_fastest,
                    "driverFastest": driver_fastest,
                    "fastestVehicleIndex": fastest_vehicle,
                    "fastestSpeedMps": fastest_speed_kph / KILOMETRES_PER_HOUR_PER_METRE_PER_SECOND
                }),
            )
        }
        b"STLG" => ("start_lights.changed", json!({"count": cursor.read_u8()?})),
        b"LGOT" => ("start_lights.out", Value::Null),
        b"DTSV" => {
            let vehicle = cursor.read_u8()?;
            (
                "penalty.drive_through_served",
                json!({"vehicleIndex": vehicle, "isPlayer": vehicle == player}),
            )
        }
        b"SGSV" => {
            let vehicle = cursor.read_u8()?;
            let stop_time = cursor.read_f32_le()?;
            finite("stop_go_time", stop_time)?;
            (
                "penalty.stop_go_served",
                json!({"vehicleIndex": vehicle, "isPlayer": vehicle == player, "stopTimeMs": seconds_to_millis_u32("stop_go_time", stop_time)?}),
            )
        }
        b"FLBK" => {
            let frame_identifier = cursor.read_u32_le()?;
            let session_time = cursor.read_f32_le()?;
            finite("flashback_session_time", session_time)?;
            (
                "flashback.activated",
                json!({"frameIdentifier": frame_identifier, "sessionTimeMs": seconds_to_millis_u32("flashback_session_time", session_time)?}),
            )
        }
        b"BUTN" => (
            "input.buttons_changed",
            json!({"status": cursor.read_u32_le()?}),
        ),
        b"RDFL" => ("race.red_flag", Value::Null),
        b"OVTK" => {
            let overtaking = cursor.read_u8()?;
            let overtaken = cursor.read_u8()?;
            (
                "race.overtake",
                json!({"overtakingVehicleIndex": overtaking, "beingOvertakenVehicleIndex": overtaken, "playerInvolved": overtaking == player || overtaken == player}),
            )
        }
        b"SCAR" => (
            "safety_car.changed",
            json!({"safetyCarType": cursor.read_u8()?, "eventType": cursor.read_u8()?}),
        ),
        b"COLL" => {
            let first = cursor.read_u8()?;
            let second = cursor.read_u8()?;
            let severity = cursor.read_u8()?;
            (
                "collision.occurred",
                json!({"vehicle1Index": first, "vehicle2Index": second, "severity": severity, "playerInvolved": first == player || second == player}),
            )
        }
        _ => {
            let code = String::from_utf8_lossy(&sample.code).into_owned();
            ("f1.event", json!({"code": code}))
        }
    };
    Ok(TelemetryEvent {
        name: name.to_owned(),
        occurred_at,
        data,
    })
}

fn map_telemetry_tyres(sample: CarTelemetrySample) -> TyreUpdate {
    let corner = |index: usize| TyreCornerState {
        surface_temperature_c: Some(f32::from(sample.tyre_surface_temperature_c[index])),
        inner_temperature_c: Some(f32::from(sample.tyre_inner_temperature_c[index])),
        pressure_pa: Some(sample.tyre_pressure_psi[index] * PASCALS_PER_PSI),
        ..TyreCornerState::default()
    };
    TyreUpdate {
        rear_left: Some(corner(0)),
        rear_right: Some(corner(1)),
        front_left: Some(corner(2)),
        front_right: Some(corner(3)),
        ..TyreUpdate::default()
    }
}

fn session_id(header: &PacketHeader) -> String {
    format!("{:016x}", header.session_uid)
}

const fn frame_id(header: &PacketHeader) -> u32 {
    header.overall_frame_identifier
}

fn normalized(field: &'static str, value: f32) -> Result<Normalized, DecodeError> {
    Normalized::new(value).map_err(|_| DecodeError::InvalidNormalizedValue { field, value })
}

fn percent_f32(field: &'static str, value: f32) -> Result<Normalized, DecodeError> {
    normalized(field, value / 100.0)
}

fn percent_u8(field: &'static str, value: u8) -> Result<Normalized, DecodeError> {
    percent_f32(field, f32::from(value))
}

fn bool_value(field: &'static str, value: u8) -> Result<bool, DecodeError> {
    match value {
        0 => Ok(false),
        1 => Ok(true),
        actual => Err(DecodeError::InvalidEnumValue { field, actual }),
    }
}

fn compound_time_ms(minutes: u8, milliseconds: u16) -> u32 {
    u32::from(minutes) * 60_000 + u32::from(milliseconds)
}

fn finite(field: &'static str, value: f32) -> Result<(), DecodeError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(DecodeError::InvalidNumericValue { field, value })
    }
}

fn seconds_to_millis_u32(field: &'static str, value: f32) -> Result<u32, DecodeError> {
    finite(field, value)?;
    let millis = f64::from(value) * 1_000.0;
    if !(0.0..=f64::from(u32::MAX)).contains(&millis) {
        return Err(DecodeError::InvalidNumericValue { field, value });
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    Ok(millis.round() as u32)
}

fn seconds_to_millis_i32(field: &'static str, value: f32) -> Result<i32, DecodeError> {
    finite(field, value)?;
    let millis = f64::from(value) * 1_000.0;
    if !(f64::from(i32::MIN)..=f64::from(i32::MAX)).contains(&millis) {
        return Err(DecodeError::InvalidNumericValue { field, value });
    }
    #[allow(clippy::cast_possible_truncation)]
    Ok(millis.round() as i32)
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

const fn session_type(value: u8) -> &'static str {
    match value {
        1 => "practice_1",
        2 => "practice_2",
        3 => "practice_3",
        4 => "practice_short",
        5 => "qualifying_1",
        6 => "qualifying_2",
        7 => "qualifying_3",
        8 => "qualifying_short",
        9 => "qualifying_one_shot",
        10 => "race",
        11 => "race_2",
        12 => "race_3",
        13 => "time_trial",
        _ => "unknown",
    }
}
