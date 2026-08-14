use std::collections::BTreeMap;

use crate::{
    AeroState, ConditionsState, DamageState, JobState, LapUpdate, LightsState, MonotonicTimestamp,
    NavigationState, SessionUpdate, TelemetrySnapshot, TelemetryUpdate, TyreCornerState,
    TyreUpdate, VehicleUpdate,
};

type FieldFrames = BTreeMap<&'static str, u32>;

/// Session-aware, per-field reducer for partial adapter updates.
#[derive(Debug, Default, Clone)]
pub struct TelemetryReducer {
    snapshot: TelemetrySnapshot,
    field_frames: FieldFrames,
    extension_frames: BTreeMap<String, u32>,
}

impl TelemetryReducer {
    /// Creates an empty reducer associated with one stable adapter id.
    #[must_use]
    pub fn with_game_id(game_id: impl Into<String>) -> Self {
        let mut reducer = Self::default();
        reducer.snapshot.meta.game_id = Some(game_id.into());
        reducer
    }

    /// Applies one partial update, returning whether observable state changed.
    ///
    /// A new session clears old game data. Frame ordering is tracked per field,
    /// allowing a late packet to populate an untouched field without replacing
    /// a newer value from another packet group.
    pub fn apply(&mut self, update: TelemetryUpdate) -> bool {
        let TelemetryUpdate {
            received_at,
            session_id,
            frame_id,
            vehicle,
            lap,
            session,
            tyres,
            conditions,
            damage,
            aero,
            navigation,
            lights,
            job,
            extensions,
        } = update;

        let mut changed = self.apply_session_identity(session_id);
        changed |= apply_vehicle(
            &mut self.field_frames,
            &mut self.snapshot,
            &vehicle,
            frame_id,
        );
        changed |= apply_lap(&mut self.field_frames, &mut self.snapshot, &lap, frame_id);
        changed |= apply_session(
            &mut self.field_frames,
            &mut self.snapshot,
            session,
            frame_id,
        );
        changed |= apply_tyres(&mut self.field_frames, &mut self.snapshot, tyres, frame_id);
        changed |= apply_conditions(
            &mut self.field_frames,
            &mut self.snapshot,
            &conditions,
            frame_id,
        );
        changed |= apply_damage(
            &mut self.field_frames,
            &mut self.snapshot,
            &damage,
            frame_id,
        );
        changed |= apply_aero(&mut self.field_frames, &mut self.snapshot, &aero, frame_id);
        changed |= apply_navigation(
            &mut self.field_frames,
            &mut self.snapshot,
            &navigation,
            frame_id,
        );
        changed |= apply_lights(
            &mut self.field_frames,
            &mut self.snapshot,
            &lights,
            frame_id,
        );
        changed |= apply_job(&mut self.field_frames, &mut self.snapshot, job, frame_id);
        changed |= apply_extensions(
            &mut self.extension_frames,
            &mut self.snapshot,
            extensions,
            frame_id,
        );

        if changed {
            self.snapshot.meta.sequence = self.snapshot.meta.sequence.saturating_add(1);
            self.snapshot.meta.captured_at = Some(latest_timestamp(
                self.snapshot.meta.captured_at,
                received_at,
            ));
        }

        changed
    }

    /// Returns the latest complete canonical snapshot.
    #[must_use]
    pub const fn snapshot(&self) -> &TelemetrySnapshot {
        &self.snapshot
    }

    /// Consumes the reducer and returns its latest snapshot.
    #[must_use]
    pub fn into_snapshot(self) -> TelemetrySnapshot {
        self.snapshot
    }

    fn apply_session_identity(&mut self, session_id: Option<String>) -> bool {
        let Some(session_id) = session_id else {
            return false;
        };

        if self.snapshot.meta.session_id.as_ref() == Some(&session_id) {
            return false;
        }

        let game_id = self.snapshot.meta.game_id.take();
        self.snapshot = TelemetrySnapshot::default();
        self.snapshot.meta.game_id = game_id;
        self.snapshot.meta.session_id = Some(session_id);
        self.field_frames.clear();
        self.extension_frames.clear();
        true
    }
}

fn apply_vehicle(
    frames: &mut FieldFrames,
    snapshot: &mut TelemetrySnapshot,
    update: &VehicleUpdate,
    frame: Option<u32>,
) -> bool {
    let mut changed = false;
    macro_rules! field {
        ($name:ident, $path:literal) => {
            changed |= apply_optional(
                frames,
                &mut snapshot.vehicle.$name,
                update.$name,
                $path,
                frame,
            );
        };
    }
    field!(speed_mps, "vehicle.speedMps");
    changed |= apply_value(
        frames,
        &mut snapshot.vehicle.gear,
        update.gear,
        "vehicle.gear",
        frame,
    );
    field!(rpm, "vehicle.rpm");
    field!(rpm_max, "vehicle.rpmMax");
    field!(rev_lights, "vehicle.revLights");
    field!(throttle, "vehicle.throttle");
    field!(brake, "vehicle.brake");
    changed |= apply_value(
        frames,
        &mut snapshot.vehicle.drs,
        update.drs,
        "vehicle.drs",
        frame,
    );
    field!(fuel_kg, "vehicle.fuelKg");
    field!(fuel_capacity_kg, "vehicle.fuelCapacityKg");
    field!(fuel_remaining_laps, "vehicle.fuelRemainingLaps");
    field!(fuel_liters, "vehicle.fuelLiters");
    field!(fuel_capacity_liters, "vehicle.fuelCapacityLiters");
    field!(fuel_range_km, "vehicle.fuelRangeKm");
    field!(fuel_warning, "vehicle.fuelWarning");
    field!(pit_limiter, "vehicle.pitLimiter");
    field!(ers_energy_j, "vehicle.ersEnergyJ");
    changed
}

fn apply_lap(
    frames: &mut FieldFrames,
    snapshot: &mut TelemetrySnapshot,
    update: &LapUpdate,
    frame: Option<u32>,
) -> bool {
    let mut changed = false;
    changed |= apply_optional(
        frames,
        &mut snapshot.lap.current,
        update.current,
        "lap.current",
        frame,
    );
    changed |= apply_optional(
        frames,
        &mut snapshot.lap.position,
        update.position,
        "lap.position",
        frame,
    );
    changed |= apply_optional(
        frames,
        &mut snapshot.lap.current_time_ms,
        update.current_time_ms,
        "lap.currentTimeMs",
        frame,
    );
    changed |= apply_optional(
        frames,
        &mut snapshot.lap.last_time_ms,
        update.last_time_ms,
        "lap.lastTimeMs",
        frame,
    );
    changed |= apply_optional(
        frames,
        &mut snapshot.lap.delta_to_best_ms,
        update.delta_to_best_ms,
        "lap.deltaToBestMs",
        frame,
    );
    changed |= apply_optional(
        frames,
        &mut snapshot.lap.invalid,
        update.invalid,
        "lap.invalid",
        frame,
    );
    macro_rules! apply_lap_field {
        ($field:ident, $path:literal) => {
            changed |= apply_optional(
                frames,
                &mut snapshot.lap.$field,
                update.$field,
                $path,
                frame,
            );
        };
    }
    apply_lap_field!(sector, "lap.sector");
    apply_lap_field!(sector1_time_ms, "lap.sector1TimeMs");
    apply_lap_field!(sector2_time_ms, "lap.sector2TimeMs");
    apply_lap_field!(delta_to_car_in_front_ms, "lap.deltaToCarInFrontMs");
    apply_lap_field!(delta_to_race_leader_ms, "lap.deltaToRaceLeaderMs");
    apply_lap_field!(distance_m, "lap.distanceM");
    apply_lap_field!(total_distance_m, "lap.totalDistanceM");
    apply_lap_field!(safety_car_delta_ms, "lap.safetyCarDeltaMs");
    apply_lap_field!(pit_status, "lap.pitStatus");
    apply_lap_field!(pit_stops, "lap.pitStops");
    apply_lap_field!(penalties_seconds, "lap.penaltiesSeconds");
    apply_lap_field!(warnings, "lap.warnings");
    apply_lap_field!(corner_cutting_warnings, "lap.cornerCuttingWarnings");
    apply_lap_field!(
        unserved_drive_through_penalties,
        "lap.unservedDriveThroughPenalties"
    );
    apply_lap_field!(unserved_stop_go_penalties, "lap.unservedStopGoPenalties");
    apply_lap_field!(grid_position, "lap.gridPosition");
    apply_lap_field!(driver_status, "lap.driverStatus");
    apply_lap_field!(result_status, "lap.resultStatus");
    apply_lap_field!(pit_lane_time_ms, "lap.pitLaneTimeMs");
    apply_lap_field!(pit_stop_time_ms, "lap.pitStopTimeMs");
    apply_lap_field!(
        pit_stop_should_serve_penalty,
        "lap.pitStopShouldServePenalty"
    );
    changed
}

fn apply_session(
    frames: &mut FieldFrames,
    snapshot: &mut TelemetrySnapshot,
    update: SessionUpdate,
    frame: Option<u32>,
) -> bool {
    let mut changed = false;
    changed |= apply_optional(
        frames,
        &mut snapshot.session.track_id,
        update.track_id,
        "session.trackId",
        frame,
    );
    changed |= apply_optional(
        frames,
        &mut snapshot.session.remaining_time_ms,
        update.remaining_time_ms,
        "session.remainingTimeMs",
        frame,
    );
    changed |= apply_optional(
        frames,
        &mut snapshot.session.total_laps,
        update.total_laps,
        "session.totalLaps",
        frame,
    );
    changed |= apply_optional(
        frames,
        &mut snapshot.session.session_type,
        update.session_type,
        "session.sessionType",
        frame,
    );
    changed |= apply_optional(
        frames,
        &mut snapshot.session.track_length_m,
        update.track_length_m,
        "session.trackLengthM",
        frame,
    );
    changed |= apply_optional(
        frames,
        &mut snapshot.session.pit_speed_limit_mps,
        update.pit_speed_limit_mps,
        "session.pitSpeedLimitMps",
        frame,
    );
    changed |= apply_optional(
        frames,
        &mut snapshot.session.safety_car_status,
        update.safety_car_status,
        "session.safetyCarStatus",
        frame,
    );
    changed |= apply_optional(
        frames,
        &mut snapshot.session.race_flag,
        update.race_flag,
        "session.raceFlag",
        frame,
    );
    changed
}

fn apply_tyres(
    frames: &mut FieldFrames,
    snapshot: &mut TelemetrySnapshot,
    update: TyreUpdate,
    frame: Option<u32>,
) -> bool {
    let mut changed = false;
    changed |= apply_tyre_corner(
        frames,
        &mut snapshot.tyres.front_left,
        update.front_left,
        TyreCorner::FrontLeft,
        frame,
    );
    changed |= apply_tyre_corner(
        frames,
        &mut snapshot.tyres.front_right,
        update.front_right,
        TyreCorner::FrontRight,
        frame,
    );
    changed |= apply_tyre_corner(
        frames,
        &mut snapshot.tyres.rear_left,
        update.rear_left,
        TyreCorner::RearLeft,
        frame,
    );
    changed |= apply_tyre_corner(
        frames,
        &mut snapshot.tyres.rear_right,
        update.rear_right,
        TyreCorner::RearRight,
        frame,
    );
    changed |= apply_optional(
        frames,
        &mut snapshot.tyres.actual_compound,
        update.actual_compound,
        "tyres.actualCompound",
        frame,
    );
    changed |= apply_optional(
        frames,
        &mut snapshot.tyres.visual_compound,
        update.visual_compound,
        "tyres.visualCompound",
        frame,
    );
    changed |= apply_optional(
        frames,
        &mut snapshot.tyres.age_laps,
        update.age_laps,
        "tyres.ageLaps",
        frame,
    );
    changed
}

#[derive(Clone, Copy)]
enum TyreCorner {
    FrontLeft,
    FrontRight,
    RearLeft,
    RearRight,
}

fn apply_tyre_corner(
    frames: &mut FieldFrames,
    target: &mut Option<TyreCornerState>,
    update: Option<TyreCornerState>,
    corner: TyreCorner,
    frame: Option<u32>,
) -> bool {
    let Some(update) = update else {
        return false;
    };
    if update.surface_temperature_c.is_none()
        && update.inner_temperature_c.is_none()
        && update.pressure_pa.is_none()
        && update.wear.is_none()
        && update.damage.is_none()
        && update.brake_damage.is_none()
        && update.blister.is_none()
    {
        return false;
    }

    let paths = match corner {
        TyreCorner::FrontLeft => [
            "tyres.frontLeft.surfaceTemperatureC",
            "tyres.frontLeft.innerTemperatureC",
            "tyres.frontLeft.pressurePa",
            "tyres.frontLeft.wear",
            "tyres.frontLeft.damage",
            "tyres.frontLeft.brakeDamage",
            "tyres.frontLeft.blister",
        ],
        TyreCorner::FrontRight => [
            "tyres.frontRight.surfaceTemperatureC",
            "tyres.frontRight.innerTemperatureC",
            "tyres.frontRight.pressurePa",
            "tyres.frontRight.wear",
            "tyres.frontRight.damage",
            "tyres.frontRight.brakeDamage",
            "tyres.frontRight.blister",
        ],
        TyreCorner::RearLeft => [
            "tyres.rearLeft.surfaceTemperatureC",
            "tyres.rearLeft.innerTemperatureC",
            "tyres.rearLeft.pressurePa",
            "tyres.rearLeft.wear",
            "tyres.rearLeft.damage",
            "tyres.rearLeft.brakeDamage",
            "tyres.rearLeft.blister",
        ],
        TyreCorner::RearRight => [
            "tyres.rearRight.surfaceTemperatureC",
            "tyres.rearRight.innerTemperatureC",
            "tyres.rearRight.pressurePa",
            "tyres.rearRight.wear",
            "tyres.rearRight.damage",
            "tyres.rearRight.brakeDamage",
            "tyres.rearRight.blister",
        ],
    };
    let mut candidate = target.clone().unwrap_or_default();
    let mut changed = false;
    changed |= apply_optional(
        frames,
        &mut candidate.surface_temperature_c,
        update.surface_temperature_c,
        paths[0],
        frame,
    );
    changed |= apply_optional(
        frames,
        &mut candidate.inner_temperature_c,
        update.inner_temperature_c,
        paths[1],
        frame,
    );
    changed |= apply_optional(
        frames,
        &mut candidate.pressure_pa,
        update.pressure_pa,
        paths[2],
        frame,
    );
    changed |= apply_optional(frames, &mut candidate.wear, update.wear, paths[3], frame);
    changed |= apply_optional(
        frames,
        &mut candidate.damage,
        update.damage,
        paths[4],
        frame,
    );
    changed |= apply_optional(
        frames,
        &mut candidate.brake_damage,
        update.brake_damage,
        paths[5],
        frame,
    );
    changed |= apply_optional(
        frames,
        &mut candidate.blister,
        update.blister,
        paths[6],
        frame,
    );
    if changed {
        *target = Some(candidate);
    }
    changed
}

fn apply_conditions(
    frames: &mut FieldFrames,
    snapshot: &mut TelemetrySnapshot,
    update: &ConditionsState,
    frame: Option<u32>,
) -> bool {
    let mut changed = false;
    changed |= apply_optional(
        frames,
        &mut snapshot.conditions.weather,
        update.weather,
        "conditions.weather",
        frame,
    );
    changed |= apply_optional(
        frames,
        &mut snapshot.conditions.track_temperature_c,
        update.track_temperature_c,
        "conditions.trackTemperatureC",
        frame,
    );
    changed |= apply_optional(
        frames,
        &mut snapshot.conditions.air_temperature_c,
        update.air_temperature_c,
        "conditions.airTemperatureC",
        frame,
    );
    changed
}

fn apply_damage(
    frames: &mut FieldFrames,
    snapshot: &mut TelemetrySnapshot,
    update: &DamageState,
    frame: Option<u32>,
) -> bool {
    let mut changed = false;
    macro_rules! field {
        ($name:ident, $path:literal) => {
            changed |= apply_optional(
                frames,
                &mut snapshot.damage.$name,
                update.$name,
                $path,
                frame,
            );
        };
    }
    field!(front_left_wing, "damage.frontLeftWing");
    field!(front_right_wing, "damage.frontRightWing");
    field!(rear_wing, "damage.rearWing");
    field!(floor, "damage.floor");
    field!(diffuser, "damage.diffuser");
    field!(sidepod, "damage.sidepod");
    field!(gearbox, "damage.gearbox");
    field!(engine, "damage.engine");
    field!(engine_mguh_wear, "damage.engineMguhWear");
    field!(engine_es_wear, "damage.engineEsWear");
    field!(engine_ce_wear, "damage.engineCeWear");
    field!(engine_ice_wear, "damage.engineIceWear");
    field!(engine_mguk_wear, "damage.engineMgukWear");
    field!(engine_tc_wear, "damage.engineTcWear");
    field!(drs_fault, "damage.drsFault");
    field!(ers_fault, "damage.ersFault");
    field!(engine_blown, "damage.engineBlown");
    field!(engine_seized, "damage.engineSeized");
    changed
}

fn apply_aero(
    frames: &mut FieldFrames,
    snapshot: &mut TelemetrySnapshot,
    update: &AeroState,
    frame: Option<u32>,
) -> bool {
    let mut changed = false;
    macro_rules! field {
        ($name:ident, $path:literal) => {
            changed |= apply_optional(frames, &mut snapshot.aero.$name, update.$name, $path, frame);
        };
    }
    field!(mode, "aero.mode");
    field!(available, "aero.available");
    field!(activation_distance_m, "aero.activationDistanceM");
    field!(overtake_available, "aero.overtakeAvailable");
    field!(overtake_active, "aero.overtakeActive");
    field!(
        overtake_activation_distance_m,
        "aero.overtakeActivationDistanceM"
    );
    field!(regulations_2026, "aero.regulations2026");
    field!(driving_wrong_way, "aero.drivingWrongWay");
    changed
}

fn apply_navigation(
    frames: &mut FieldFrames,
    snapshot: &mut TelemetrySnapshot,
    update: &NavigationState,
    frame: Option<u32>,
) -> bool {
    let mut changed = false;
    changed |= apply_optional(
        frames,
        &mut snapshot.navigation.distance_m,
        update.distance_m,
        "navigation.distanceM",
        frame,
    );
    changed |= apply_optional(
        frames,
        &mut snapshot.navigation.time_s,
        update.time_s,
        "navigation.timeS",
        frame,
    );
    changed |= apply_optional(
        frames,
        &mut snapshot.navigation.speed_limit_mps,
        update.speed_limit_mps,
        "navigation.speedLimitMps",
        frame,
    );
    changed
}

fn apply_lights(
    frames: &mut FieldFrames,
    snapshot: &mut TelemetrySnapshot,
    update: &LightsState,
    frame: Option<u32>,
) -> bool {
    let mut changed = false;
    macro_rules! field {
        ($name:ident, $path:literal) => {
            changed |= apply_optional(
                frames,
                &mut snapshot.lights.$name,
                update.$name,
                $path,
                frame,
            );
        };
    }
    field!(parking, "lights.parking");
    field!(low_beam, "lights.lowBeam");
    field!(high_beam, "lights.highBeam");
    field!(beacon, "lights.beacon");
    field!(brake, "lights.brake");
    field!(reverse, "lights.reverse");
    field!(left_indicator, "lights.leftIndicator");
    field!(right_indicator, "lights.rightIndicator");
    field!(hazard, "lights.hazard");
    changed
}

fn apply_job(
    frames: &mut FieldFrames,
    snapshot: &mut TelemetrySnapshot,
    update: JobState,
    frame: Option<u32>,
) -> bool {
    if update.active == Some(false) {
        if !should_apply(frames, "job.active", frame) {
            return false;
        }
        if let Some(frame) = frame {
            for field in [
                "job.cargo",
                "job.cargoMassKg",
                "job.sourceCity",
                "job.destinationCity",
                "job.income",
                "job.deliveryTime",
                "job.plannedDistanceKm",
                "job.cargoLoaded",
                "job.special",
            ] {
                frames.insert(field, frame);
            }
        }
        snapshot.job = JobState {
            active: Some(false),
            ..JobState::default()
        };
        return true;
    }

    let mut changed = false;
    if update.active.is_some() && !should_apply(frames, "job.active", frame) {
        return false;
    }
    macro_rules! field {
        ($name:ident, $path:literal) => {
            changed |= apply_optional(frames, &mut snapshot.job.$name, update.$name, $path, frame);
        };
    }
    if let Some(active) = update.active {
        snapshot.job.active = Some(active);
        changed = true;
    }
    field!(cargo, "job.cargo");
    field!(cargo_mass_kg, "job.cargoMassKg");
    field!(source_city, "job.sourceCity");
    field!(destination_city, "job.destinationCity");
    field!(income, "job.income");
    field!(delivery_time, "job.deliveryTime");
    field!(planned_distance_km, "job.plannedDistanceKm");
    field!(cargo_loaded, "job.cargoLoaded");
    field!(special, "job.special");
    changed
}

fn apply_optional<T>(
    frames: &mut FieldFrames,
    target: &mut Option<T>,
    value: Option<T>,
    field: &'static str,
    frame: Option<u32>,
) -> bool {
    let Some(value) = value else {
        return false;
    };
    if !should_apply(frames, field, frame) {
        return false;
    }
    *target = Some(value);
    true
}

fn apply_value<T>(
    frames: &mut FieldFrames,
    target: &mut T,
    value: Option<T>,
    field: &'static str,
    frame: Option<u32>,
) -> bool {
    let Some(value) = value else {
        return false;
    };
    if !should_apply(frames, field, frame) {
        return false;
    }
    *target = value;
    true
}

fn should_apply(frames: &mut FieldFrames, field: &'static str, frame: Option<u32>) -> bool {
    let Some(candidate) = frame else {
        return true;
    };
    if frames
        .get(field)
        .is_some_and(|current| !frame_is_at_least_as_new(candidate, *current))
    {
        return false;
    }
    frames.insert(field, candidate);
    true
}

fn apply_extensions(
    frames: &mut BTreeMap<String, u32>,
    snapshot: &mut TelemetrySnapshot,
    extensions: BTreeMap<String, serde_json::Value>,
    frame: Option<u32>,
) -> bool {
    let mut changed = false;
    for (adapter_id, value) in extensions {
        let is_current = frame.is_none_or(|candidate| {
            frames
                .get(&adapter_id)
                .is_none_or(|current| frame_is_at_least_as_new(candidate, *current))
        });
        if !is_current {
            continue;
        }
        if let Some(frame) = frame {
            frames.insert(adapter_id.clone(), frame);
        }
        snapshot.extensions.insert(adapter_id, value);
        changed = true;
    }
    changed
}

fn frame_is_at_least_as_new(candidate: u32, current: u32) -> bool {
    candidate == current || candidate.wrapping_sub(current) < (1_u32 << 31)
}

fn latest_timestamp(
    current: Option<MonotonicTimestamp>,
    candidate: MonotonicTimestamp,
) -> MonotonicTimestamp {
    current.map_or(candidate, |current| current.max(candidate))
}
