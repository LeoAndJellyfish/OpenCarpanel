use std::{
    collections::BTreeMap,
    error::Error,
    fmt::{self, Display, Formatter},
    num::NonZeroU8,
};

use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, de::Error as _};
use serde_json::Value;

/// Current major version of the game-neutral telemetry schema.
pub const TELEMETRY_SCHEMA_VERSION: u16 = 1;

/// A finite value in the inclusive range `0.0..=1.0`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, JsonSchema)]
#[serde(transparent)]
#[schemars(transparent)]
pub struct Normalized(#[schemars(range(min = 0.0, max = 1.0))] f32);

impl Normalized {
    /// Creates a normalized value.
    ///
    /// # Errors
    ///
    /// Returns [`NormalizedError`] when `value` is not finite or lies outside
    /// the inclusive unit interval.
    pub fn new(value: f32) -> Result<Self, NormalizedError> {
        if value.is_finite() && (0.0..=1.0).contains(&value) {
            Ok(Self(value))
        } else {
            Err(NormalizedError { value })
        }
    }

    /// Returns the primitive value.
    #[must_use]
    pub const fn get(self) -> f32 {
        self.0
    }
}

impl<'de> Deserialize<'de> for Normalized {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = f32::deserialize(deserializer)?;
        Self::new(value).map_err(D::Error::custom)
    }
}

/// Error returned when a value cannot be represented as [`Normalized`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NormalizedError {
    value: f32,
}

impl NormalizedError {
    /// Returns the rejected primitive value.
    #[must_use]
    pub const fn value(self) -> f32 {
        self.value
    }
}

impl Display for NormalizedError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "normalized value must be finite and within 0.0..=1.0, got {}",
            self.value
        )
    }
}

impl Error for NormalizedError {}

/// Microseconds elapsed from the Host's monotonic clock origin.
#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
#[schemars(transparent)]
pub struct MonotonicTimestamp(u64);

impl MonotonicTimestamp {
    /// Creates a timestamp from microseconds elapsed since the Host clock origin.
    #[must_use]
    pub const fn from_micros(micros: u64) -> Self {
        Self(micros)
    }

    /// Returns microseconds elapsed since the Host clock origin.
    #[must_use]
    pub const fn as_micros(self) -> u64 {
        self.0
    }
}

/// Gear selected by the game.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Gear {
    /// Reverse gear.
    Reverse,
    /// Neutral.
    Neutral,
    /// A forward gear numbered from one.
    Forward(NonZeroU8),
    /// The game has not supplied a meaningful gear.
    #[default]
    Unknown,
}

impl Gear {
    /// Creates a forward gear, returning `None` for zero.
    #[must_use]
    pub fn forward(number: u8) -> Option<Self> {
        NonZeroU8::new(number).map(Self::Forward)
    }

    #[allow(clippy::trivially_copy_pass_by_ref)]
    fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown)
    }
}

/// Driver-reduction-system state normalized across supported games.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DrsState {
    /// The game or vehicle does not support DRS in the current context.
    Unavailable,
    /// DRS can be activated but is currently closed.
    Available,
    /// DRS is currently active.
    Active,
    /// The game has not supplied a meaningful DRS state.
    #[default]
    Unknown,
}

impl DrsState {
    #[allow(clippy::trivially_copy_pass_by_ref)]
    fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown)
    }
}

/// Metadata attached to every complete telemetry snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct Meta {
    /// Schema version used by this snapshot.
    pub schema_version: u16,
    /// Stable adapter identifier, such as `f1-24`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub game_id: Option<String>,
    /// Opaque identifier for the active game session.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// Monotonic snapshot sequence assigned by the core.
    pub sequence: u64,
    /// Time at which the newest contributing datagram reached the Host.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub captured_at: Option<MonotonicTimestamp>,
}

impl Default for Meta {
    fn default() -> Self {
        Self {
            schema_version: TELEMETRY_SCHEMA_VERSION,
            game_id: None,
            session_id: None,
            sequence: 0,
            captured_at: None,
        }
    }
}

/// Current player-vehicle state in standard units.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct VehicleState {
    /// Vehicle speed in metres per second.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed_mps: Option<f32>,
    /// Selected gear.
    #[serde(skip_serializing_if = "Gear::is_unknown")]
    pub gear: Gear,
    /// Current engine revolutions per minute.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rpm: Option<u16>,
    /// Maximum engine revolutions per minute used for display scaling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rpm_max: Option<u16>,
    /// Accelerator input in the inclusive unit interval.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub throttle: Option<Normalized>,
    /// Brake input in the inclusive unit interval.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brake: Option<Normalized>,
    /// Driver-reduction-system state.
    #[serde(skip_serializing_if = "DrsState::is_unknown")]
    pub drs: DrsState,
    /// Remaining fuel mass in kilograms.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fuel_kg: Option<f32>,
    /// Available electrical energy in joules.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ers_energy_j: Option<f32>,
}

/// Current lap and race-position state.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct LapState {
    /// Current one-based lap number.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current: Option<u16>,
    /// Current one-based race position.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<u8>,
    /// Elapsed current-lap time in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_time_ms: Option<u32>,
    /// Previous completed-lap time in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_time_ms: Option<u32>,
    /// Signed delta to the relevant best lap in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta_to_best_ms: Option<i32>,
    /// Whether the current lap has been invalidated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invalid: Option<bool>,
}

/// Current session state that is meaningful across games.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct SessionState {
    /// Stable game-provided or mapped track identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track_id: Option<String>,
    /// Remaining session time in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remaining_time_ms: Option<u64>,
    /// Scheduled total number of laps when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_laps: Option<u16>,
}

/// State for one tyre corner.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct TyreCornerState {
    /// Tyre surface temperature in degrees Celsius.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub surface_temperature_c: Option<f32>,
    /// Tyre inner temperature in degrees Celsius.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inner_temperature_c: Option<f32>,
    /// Tyre pressure in pascals.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pressure_pa: Option<f32>,
    /// Tyre wear in the inclusive unit interval.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wear: Option<Normalized>,
}

/// State for all four tyre corners.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct TyreState {
    /// Front-left tyre.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub front_left: Option<TyreCornerState>,
    /// Front-right tyre.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub front_right: Option<TyreCornerState>,
    /// Rear-left tyre.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rear_left: Option<TyreCornerState>,
    /// Rear-right tyre.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rear_right: Option<TyreCornerState>,
}

/// A complete, game-neutral view of the latest known telemetry state.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct TelemetrySnapshot {
    /// Snapshot metadata.
    pub meta: Meta,
    /// Player-vehicle state.
    pub vehicle: VehicleState,
    /// Lap and race-position state.
    pub lap: LapState,
    /// Session state.
    pub session: SessionState,
    /// Four-corner tyre state.
    pub tyres: TyreState,
    /// Adapter-specific values that do not yet have stable cross-game semantics.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub extensions: BTreeMap<String, Value>,
}

/// A discrete telemetry fact that must not be treated as replaceable state.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TelemetryEvent {
    /// Stable dotted event name, such as `lap.completed`.
    pub name: String,
    /// Host-monotonic time at which the event was observed.
    pub occurred_at: MonotonicTimestamp,
    /// Event-specific structured data.
    #[serde(default)]
    pub data: Value,
}

/// Stable fields that an adapter can advertise to clients.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
pub enum TelemetryField {
    /// Vehicle speed in metres per second.
    #[serde(rename = "vehicle.speedMps")]
    VehicleSpeed,
    /// Selected gear.
    #[serde(rename = "vehicle.gear")]
    VehicleGear,
    /// Current engine revolutions per minute.
    #[serde(rename = "vehicle.rpm")]
    VehicleRpm,
    /// Maximum engine revolutions per minute.
    #[serde(rename = "vehicle.rpmMax")]
    VehicleRpmMax,
    /// Accelerator input.
    #[serde(rename = "vehicle.throttle")]
    VehicleThrottle,
    /// Brake input.
    #[serde(rename = "vehicle.brake")]
    VehicleBrake,
    /// Driver-reduction-system state.
    #[serde(rename = "vehicle.drs")]
    VehicleDrs,
    /// Current lap number.
    #[serde(rename = "lap.current")]
    LapCurrent,
    /// Current race position.
    #[serde(rename = "lap.position")]
    LapPosition,
    /// Current-lap time.
    #[serde(rename = "lap.currentTimeMs")]
    LapCurrentTime,
    /// Previous completed-lap time.
    #[serde(rename = "lap.lastTimeMs")]
    LapLastTime,
    /// Delta to the relevant best lap.
    #[serde(rename = "lap.deltaToBestMs")]
    LapDeltaToBest,
    /// Current-lap invalidation state.
    #[serde(rename = "lap.invalid")]
    LapInvalid,
    /// Track identifier.
    #[serde(rename = "session.trackId")]
    SessionTrack,
    /// Remaining session time.
    #[serde(rename = "session.remainingTimeMs")]
    SessionRemainingTime,
    /// Scheduled total laps.
    #[serde(rename = "session.totalLaps")]
    SessionTotalLaps,
    /// Four-corner tyre state.
    #[serde(rename = "tyres")]
    Tyres,
}

impl TelemetryField {
    /// Returns the stable dotted path used by widget manifests and capabilities.
    #[must_use]
    pub const fn as_path(self) -> &'static str {
        match self {
            Self::VehicleSpeed => "vehicle.speedMps",
            Self::VehicleGear => "vehicle.gear",
            Self::VehicleRpm => "vehicle.rpm",
            Self::VehicleRpmMax => "vehicle.rpmMax",
            Self::VehicleThrottle => "vehicle.throttle",
            Self::VehicleBrake => "vehicle.brake",
            Self::VehicleDrs => "vehicle.drs",
            Self::LapCurrent => "lap.current",
            Self::LapPosition => "lap.position",
            Self::LapCurrentTime => "lap.currentTimeMs",
            Self::LapLastTime => "lap.lastTimeMs",
            Self::LapDeltaToBest => "lap.deltaToBestMs",
            Self::LapInvalid => "lap.invalid",
            Self::SessionTrack => "session.trackId",
            Self::SessionRemainingTime => "session.remainingTimeMs",
            Self::SessionTotalLaps => "session.totalLaps",
            Self::Tyres => "tyres",
        }
    }
}

/// Partial player-vehicle state emitted by an adapter.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct VehicleUpdate {
    /// New speed when this packet updates speed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed_mps: Option<f32>,
    /// New gear when this packet updates gear.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gear: Option<Gear>,
    /// New engine speed when this packet updates engine speed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rpm: Option<u16>,
    /// New maximum engine speed when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rpm_max: Option<u16>,
    /// New accelerator input when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub throttle: Option<Normalized>,
    /// New brake input when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brake: Option<Normalized>,
    /// New DRS state when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drs: Option<DrsState>,
    /// New fuel mass when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fuel_kg: Option<f32>,
    /// New electrical energy level when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ers_energy_j: Option<f32>,
}

/// Partial lap state emitted by an adapter.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct LapUpdate {
    /// New current lap number when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current: Option<u16>,
    /// New race position when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<u8>,
    /// New current-lap time when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_time_ms: Option<u32>,
    /// New previous-lap time when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_time_ms: Option<u32>,
    /// New delta to the relevant best lap when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta_to_best_ms: Option<i32>,
    /// New lap invalidation state when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invalid: Option<bool>,
}

/// Partial session state emitted by an adapter.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct SessionUpdate {
    /// New track identifier when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track_id: Option<String>,
    /// New remaining session time when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remaining_time_ms: Option<u64>,
    /// New scheduled total lap count when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_laps: Option<u16>,
}

/// Partial four-corner tyre state emitted by an adapter.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct TyreUpdate {
    /// New front-left state when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub front_left: Option<TyreCornerState>,
    /// New front-right state when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub front_right: Option<TyreCornerState>,
    /// New rear-left state when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rear_left: Option<TyreCornerState>,
    /// New rear-right state when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rear_right: Option<TyreCornerState>,
}

/// Partial game-neutral state emitted after decoding one game datagram.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct TelemetryUpdate {
    /// Host-monotonic time at which the datagram was received.
    pub received_at: MonotonicTimestamp,
    /// New game session identifier when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// Game-provided frame identifier when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame_id: Option<u32>,
    /// Partial player-vehicle update.
    pub vehicle: VehicleUpdate,
    /// Partial lap update.
    pub lap: LapUpdate,
    /// Partial session update.
    pub session: SessionUpdate,
    /// Partial tyre update.
    pub tyres: TyreUpdate,
    /// Adapter-specific partial values.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub extensions: BTreeMap<String, Value>,
}
