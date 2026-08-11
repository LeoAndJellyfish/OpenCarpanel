//! Game-neutral telemetry state, events, capabilities, and session lifecycle.

mod model;

pub use model::{
    DrsState, Gear, LapState, LapUpdate, Meta, MonotonicTimestamp, Normalized, NormalizedError,
    SessionState, SessionUpdate, TELEMETRY_SCHEMA_VERSION, TelemetryEvent, TelemetryField,
    TelemetrySnapshot, TelemetryUpdate, TyreCornerState, TyreState, TyreUpdate, VehicleState,
    VehicleUpdate,
};
