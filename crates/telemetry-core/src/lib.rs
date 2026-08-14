//! Game-neutral telemetry state, events, capabilities, and session lifecycle.

mod model;
mod reducer;

pub use model::{
    ActiveAeroMode, AeroState, ConditionsState, DamageState, DriverStatus, DrsState, Gear,
    JobState, LapState, LapUpdate, LightsState, Meta, MonotonicTimestamp, NavigationState,
    Normalized, NormalizedError, PitStatus, RaceFlag, ResultStatus, SafetyCarStatus, SessionState,
    SessionUpdate, TELEMETRY_SCHEMA_VERSION, TelemetryEvent, TelemetryField, TelemetrySnapshot,
    TelemetryUpdate, TyreCornerState, TyreState, TyreUpdate, VehicleState, VehicleUpdate,
    WeatherCondition,
};
pub use reducer::TelemetryReducer;
