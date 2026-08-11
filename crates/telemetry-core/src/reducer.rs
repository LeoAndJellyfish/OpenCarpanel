use std::collections::BTreeMap;

use crate::{
    LapUpdate, MonotonicTimestamp, SessionUpdate, TelemetrySnapshot, TelemetryUpdate, TyreUpdate,
    VehicleUpdate,
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
    changed |= apply_optional(
        frames,
        &mut snapshot.vehicle.speed_mps,
        update.speed_mps,
        "vehicle.speedMps",
        frame,
    );
    changed |= apply_value(
        frames,
        &mut snapshot.vehicle.gear,
        update.gear,
        "vehicle.gear",
        frame,
    );
    changed |= apply_optional(
        frames,
        &mut snapshot.vehicle.rpm,
        update.rpm,
        "vehicle.rpm",
        frame,
    );
    changed |= apply_optional(
        frames,
        &mut snapshot.vehicle.rpm_max,
        update.rpm_max,
        "vehicle.rpmMax",
        frame,
    );
    changed |= apply_optional(
        frames,
        &mut snapshot.vehicle.rev_lights,
        update.rev_lights,
        "vehicle.revLights",
        frame,
    );
    changed |= apply_optional(
        frames,
        &mut snapshot.vehicle.throttle,
        update.throttle,
        "vehicle.throttle",
        frame,
    );
    changed |= apply_optional(
        frames,
        &mut snapshot.vehicle.brake,
        update.brake,
        "vehicle.brake",
        frame,
    );
    changed |= apply_value(
        frames,
        &mut snapshot.vehicle.drs,
        update.drs,
        "vehicle.drs",
        frame,
    );
    changed |= apply_optional(
        frames,
        &mut snapshot.vehicle.fuel_kg,
        update.fuel_kg,
        "vehicle.fuelKg",
        frame,
    );
    changed |= apply_optional(
        frames,
        &mut snapshot.vehicle.ers_energy_j,
        update.ers_energy_j,
        "vehicle.ersEnergyJ",
        frame,
    );
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
    changed
}

fn apply_tyres(
    frames: &mut FieldFrames,
    snapshot: &mut TelemetrySnapshot,
    update: TyreUpdate,
    frame: Option<u32>,
) -> bool {
    let mut changed = false;
    changed |= apply_optional(
        frames,
        &mut snapshot.tyres.front_left,
        update.front_left,
        "tyres.frontLeft",
        frame,
    );
    changed |= apply_optional(
        frames,
        &mut snapshot.tyres.front_right,
        update.front_right,
        "tyres.frontRight",
        frame,
    );
    changed |= apply_optional(
        frames,
        &mut snapshot.tyres.rear_left,
        update.rear_left,
        "tyres.rearLeft",
        frame,
    );
    changed |= apply_optional(
        frames,
        &mut snapshot.tyres.rear_right,
        update.rear_right,
        "tyres.rearRight",
        frame,
    );
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
