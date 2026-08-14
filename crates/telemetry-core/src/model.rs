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

/// Weather reported for the current session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WeatherCondition {
    /// Clear sky.
    Clear,
    /// Light cloud cover.
    LightCloud,
    /// Overcast conditions.
    Overcast,
    /// Light rain.
    LightRain,
    /// Heavy rain.
    HeavyRain,
    /// Storm conditions.
    Storm,
}

/// Race-control flag applying to the player vehicle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RaceFlag {
    /// No flag is active.
    None,
    /// Green flag.
    Green,
    /// Blue flag.
    Blue,
    /// Yellow flag.
    Yellow,
    /// Red flag.
    Red,
}

/// Safety-car state for the current session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SafetyCarStatus {
    /// No safety car is active.
    None,
    /// Full safety car.
    Full,
    /// Virtual safety car.
    Virtual,
    /// Formation-lap safety car.
    FormationLap,
}

/// Player pit state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PitStatus {
    /// The vehicle is not pitting.
    None,
    /// The vehicle is entering or approaching the pits.
    Pitting,
    /// The vehicle is inside the pit area.
    InPitArea,
}

/// Player activity within the current lap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DriverStatus {
    /// In the garage.
    InGarage,
    /// On a flying lap.
    FlyingLap,
    /// On an in lap.
    InLap,
    /// On an out lap.
    OutLap,
    /// Driving on track outside a special lap phase.
    OnTrack,
}

/// Classification state for the player vehicle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ResultStatus {
    /// Invalid or unavailable result.
    Invalid,
    /// Inactive participant.
    Inactive,
    /// Active participant.
    Active,
    /// Finished the session.
    Finished,
    /// Did not finish.
    DidNotFinish,
    /// Disqualified.
    Disqualified,
    /// Not classified.
    NotClassified,
    /// Retired.
    Retired,
}

/// 2026 active-aerodynamics mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ActiveAeroMode {
    /// High-downforce corner mode.
    Corner,
    /// Low-drag straight mode.
    Straight,
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
    /// Game-provided shift-light progression in the inclusive unit interval.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rev_lights: Option<Normalized>,
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
    /// Fuel-tank capacity in kilograms when the game reports fuel by mass.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fuel_capacity_kg: Option<f32>,
    /// Estimated laps remaining on the current fuel load.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fuel_remaining_laps: Option<f32>,
    /// Remaining fuel volume in litres.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fuel_liters: Option<f32>,
    /// Fuel-tank capacity in litres.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fuel_capacity_liters: Option<f32>,
    /// Estimated remaining fuel range in kilometres.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fuel_range_km: Option<f32>,
    /// Whether the game is showing a low-fuel warning.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fuel_warning: Option<bool>,
    /// Whether the pit-lane speed limiter is active.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pit_limiter: Option<bool>,
    /// Available electrical energy in joules.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ers_energy_j: Option<f32>,
}

/// Current lap and race-position state.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
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
    /// Current one-based sector number.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sector: Option<u8>,
    /// Completed sector-one time in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sector1_time_ms: Option<u32>,
    /// Completed sector-two time in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sector2_time_ms: Option<u32>,
    /// Delta to the car immediately ahead in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta_to_car_in_front_ms: Option<u32>,
    /// Delta to the race leader in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta_to_race_leader_ms: Option<u32>,
    /// Distance around the current lap in metres.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distance_m: Option<f32>,
    /// Total distance travelled in the session in metres.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_distance_m: Option<f32>,
    /// Signed safety-car delta in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_car_delta_ms: Option<i32>,
    /// Current pit state.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pit_status: Option<PitStatus>,
    /// Pit stops completed in the current race.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pit_stops: Option<u8>,
    /// Accumulated time penalties in seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub penalties_seconds: Option<u8>,
    /// Accumulated warnings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warnings: Option<u8>,
    /// Accumulated corner-cutting warnings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub corner_cutting_warnings: Option<u8>,
    /// Unserved drive-through penalties.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unserved_drive_through_penalties: Option<u8>,
    /// Unserved stop-go penalties.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unserved_stop_go_penalties: Option<u8>,
    /// Starting grid position.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grid_position: Option<u8>,
    /// Current driver activity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub driver_status: Option<DriverStatus>,
    /// Current classification state.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_status: Option<ResultStatus>,
    /// Time spent in the pit lane in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pit_lane_time_ms: Option<u32>,
    /// Current pit-stop time in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pit_stop_time_ms: Option<u32>,
    /// Whether a penalty should be served during the current pit stop.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pit_stop_should_serve_penalty: Option<bool>,
}

/// Current session state that is meaningful across games.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
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
    /// Human-readable stable session type, such as `race` or `qualifying_1`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_type: Option<String>,
    /// Track length in metres.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track_length_m: Option<u32>,
    /// Pit-lane speed limit in metres per second.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pit_speed_limit_mps: Option<f32>,
    /// Current safety-car state.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_car_status: Option<SafetyCarStatus>,
    /// Current FIA or race-control flag for the player vehicle.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub race_flag: Option<RaceFlag>,
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
    /// Tyre damage in the inclusive unit interval.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub damage: Option<Normalized>,
    /// Brake damage at this corner in the inclusive unit interval.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brake_damage: Option<Normalized>,
    /// Tyre blistering in the inclusive unit interval.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blister: Option<Normalized>,
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
    /// Game-defined actual compound identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual_compound: Option<u8>,
    /// Game-defined visual compound identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visual_compound: Option<u8>,
    /// Age of the fitted tyre set in laps.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub age_laps: Option<u8>,
}

/// Current ambient and track conditions.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct ConditionsState {
    /// Current weather category.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weather: Option<WeatherCondition>,
    /// Track temperature in degrees Celsius.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track_temperature_c: Option<f32>,
    /// Air temperature in degrees Celsius.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub air_temperature_c: Option<f32>,
}

impl ConditionsState {
    fn is_empty(&self) -> bool {
        self.weather.is_none()
            && self.track_temperature_c.is_none()
            && self.air_temperature_c.is_none()
    }
}

/// Player-vehicle damage not naturally represented by one tyre corner.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct DamageState {
    /// Front-left wing damage.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub front_left_wing: Option<Normalized>,
    /// Front-right wing damage.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub front_right_wing: Option<Normalized>,
    /// Rear-wing damage.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rear_wing: Option<Normalized>,
    /// Floor damage.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub floor: Option<Normalized>,
    /// Diffuser damage.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diffuser: Option<Normalized>,
    /// Sidepod damage.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sidepod: Option<Normalized>,
    /// Gearbox damage.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gearbox: Option<Normalized>,
    /// Overall engine damage.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engine: Option<Normalized>,
    /// MGU-H wear.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engine_mguh_wear: Option<Normalized>,
    /// Energy-store wear.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engine_es_wear: Option<Normalized>,
    /// Control-electronics wear.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engine_ce_wear: Option<Normalized>,
    /// Internal-combustion-engine wear.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engine_ice_wear: Option<Normalized>,
    /// MGU-K wear.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engine_mguk_wear: Option<Normalized>,
    /// Turbocharger wear.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engine_tc_wear: Option<Normalized>,
    /// DRS system fault state.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drs_fault: Option<bool>,
    /// ERS system fault state.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ers_fault: Option<bool>,
    /// Whether the engine has blown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engine_blown: Option<bool>,
    /// Whether the engine has seized.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub engine_seized: Option<bool>,
}

impl DamageState {
    fn is_empty(&self) -> bool {
        self.front_left_wing.is_none()
            && self.front_right_wing.is_none()
            && self.rear_wing.is_none()
            && self.floor.is_none()
            && self.diffuser.is_none()
            && self.sidepod.is_none()
            && self.gearbox.is_none()
            && self.engine.is_none()
            && self.engine_mguh_wear.is_none()
            && self.engine_es_wear.is_none()
            && self.engine_ce_wear.is_none()
            && self.engine_ice_wear.is_none()
            && self.engine_mguk_wear.is_none()
            && self.engine_tc_wear.is_none()
            && self.drs_fault.is_none()
            && self.ers_fault.is_none()
            && self.engine_blown.is_none()
            && self.engine_seized.is_none()
    }
}

/// 2026 active-aerodynamics and overtake state.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct AeroState {
    /// Current active-aero mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<ActiveAeroMode>,
    /// Whether active aero is currently available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub available: Option<bool>,
    /// Distance until active aero becomes available in metres.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activation_distance_m: Option<u32>,
    /// Whether overtake mode is available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overtake_available: Option<bool>,
    /// Whether overtake mode is active.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overtake_active: Option<bool>,
    /// Distance until overtake mode becomes available in metres.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overtake_activation_distance_m: Option<u32>,
    /// Whether the car uses the 2026 regulations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub regulations_2026: Option<bool>,
    /// Whether the game reports the car is driving the wrong way.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub driving_wrong_way: Option<bool>,
}

impl AeroState {
    fn is_empty(&self) -> bool {
        self.mode.is_none()
            && self.available.is_none()
            && self.activation_distance_m.is_none()
            && self.overtake_available.is_none()
            && self.overtake_active.is_none()
            && self.overtake_activation_distance_m.is_none()
            && self.regulations_2026.is_none()
            && self.driving_wrong_way.is_none()
    }
}

/// Route-advisor state for truck simulators.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct NavigationState {
    /// Remaining route distance in metres.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distance_m: Option<f32>,
    /// Estimated route time in seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_s: Option<f32>,
    /// Current navigation speed limit in metres per second.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed_limit_mps: Option<f32>,
}

impl NavigationState {
    fn is_empty(&self) -> bool {
        self.distance_m.is_none() && self.time_s.is_none() && self.speed_limit_mps.is_none()
    }
}

/// Exterior-light state for truck simulators.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct LightsState {
    /// Parking lights.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parking: Option<bool>,
    /// Low-beam headlights.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub low_beam: Option<bool>,
    /// High-beam headlights.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub high_beam: Option<bool>,
    /// Beacon lights.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub beacon: Option<bool>,
    /// Brake lights.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub brake: Option<bool>,
    /// Reverse lights.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reverse: Option<bool>,
    /// Logical left indicator state.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left_indicator: Option<bool>,
    /// Logical right indicator state.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right_indicator: Option<bool>,
    /// Hazard-warning state.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hazard: Option<bool>,
}

impl LightsState {
    fn is_empty(&self) -> bool {
        self.parking.is_none()
            && self.low_beam.is_none()
            && self.high_beam.is_none()
            && self.beacon.is_none()
            && self.brake.is_none()
            && self.reverse.is_none()
            && self.left_indicator.is_none()
            && self.right_indicator.is_none()
            && self.hazard.is_none()
    }
}

/// Active truck-delivery job.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default, rename_all = "camelCase")]
pub struct JobState {
    /// Whether a job is active.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
    /// Localized cargo name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cargo: Option<String>,
    /// Cargo mass in kilograms.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cargo_mass_kg: Option<f32>,
    /// Localized source city.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_city: Option<String>,
    /// Localized destination city.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination_city: Option<String>,
    /// Expected income in the game's native currency.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub income: Option<u64>,
    /// Absolute in-game delivery deadline.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delivery_time: Option<u32>,
    /// Planned distance in simulated kilometres.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub planned_distance_km: Option<u32>,
    /// Whether cargo is loaded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cargo_loaded: Option<bool>,
    /// Whether this is a special-transport job.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub special: Option<bool>,
}

impl JobState {
    fn is_empty(&self) -> bool {
        self.active.is_none()
            && self.cargo.is_none()
            && self.cargo_mass_kg.is_none()
            && self.source_city.is_none()
            && self.destination_city.is_none()
            && self.income.is_none()
            && self.delivery_time.is_none()
            && self.planned_distance_km.is_none()
            && self.cargo_loaded.is_none()
            && self.special.is_none()
    }
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
    /// Ambient and track conditions.
    #[serde(skip_serializing_if = "ConditionsState::is_empty")]
    pub conditions: ConditionsState,
    /// Player-vehicle damage.
    #[serde(skip_serializing_if = "DamageState::is_empty")]
    pub damage: DamageState,
    /// Active-aerodynamics state.
    #[serde(skip_serializing_if = "AeroState::is_empty")]
    pub aero: AeroState,
    /// Route-advisor state.
    #[serde(skip_serializing_if = "NavigationState::is_empty")]
    pub navigation: NavigationState,
    /// Exterior-light state.
    #[serde(skip_serializing_if = "LightsState::is_empty")]
    pub lights: LightsState,
    /// Active delivery job.
    #[serde(skip_serializing_if = "JobState::is_empty")]
    pub job: JobState,
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
    /// Game-provided shift-light progression.
    #[serde(rename = "vehicle.revLights")]
    VehicleRevLights,
    /// Accelerator input.
    #[serde(rename = "vehicle.throttle")]
    VehicleThrottle,
    /// Brake input.
    #[serde(rename = "vehicle.brake")]
    VehicleBrake,
    /// Driver-reduction-system state.
    #[serde(rename = "vehicle.drs")]
    VehicleDrs,
    /// Fuel state in game-native mass or volume units.
    #[serde(rename = "vehicle.fuel")]
    VehicleFuel,
    /// Pit-lane limiter state.
    #[serde(rename = "vehicle.pitLimiter")]
    VehiclePitLimiter,
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
    /// Extended sector timing and race-position state.
    #[serde(rename = "lap.raceState")]
    LapRaceState,
    /// Track identifier.
    #[serde(rename = "session.trackId")]
    SessionTrack,
    /// Remaining session time.
    #[serde(rename = "session.remainingTimeMs")]
    SessionRemainingTime,
    /// Scheduled total laps.
    #[serde(rename = "session.totalLaps")]
    SessionTotalLaps,
    /// Extended session and race-control state.
    #[serde(rename = "session.raceState")]
    SessionRaceState,
    /// Four-corner tyre state.
    #[serde(rename = "tyres")]
    Tyres,
    /// Ambient and track conditions.
    #[serde(rename = "conditions")]
    Conditions,
    /// Vehicle damage state.
    #[serde(rename = "damage")]
    Damage,
    /// Active-aerodynamics state.
    #[serde(rename = "aero")]
    Aero,
    /// Route-advisor state.
    #[serde(rename = "navigation")]
    Navigation,
    /// Exterior-light state.
    #[serde(rename = "lights")]
    Lights,
    /// Delivery-job state.
    #[serde(rename = "job")]
    Job,
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
            Self::VehicleRevLights => "vehicle.revLights",
            Self::VehicleThrottle => "vehicle.throttle",
            Self::VehicleBrake => "vehicle.brake",
            Self::VehicleDrs => "vehicle.drs",
            Self::VehicleFuel => "vehicle.fuel",
            Self::VehiclePitLimiter => "vehicle.pitLimiter",
            Self::LapCurrent => "lap.current",
            Self::LapPosition => "lap.position",
            Self::LapCurrentTime => "lap.currentTimeMs",
            Self::LapLastTime => "lap.lastTimeMs",
            Self::LapDeltaToBest => "lap.deltaToBestMs",
            Self::LapInvalid => "lap.invalid",
            Self::LapRaceState => "lap.raceState",
            Self::SessionTrack => "session.trackId",
            Self::SessionRemainingTime => "session.remainingTimeMs",
            Self::SessionTotalLaps => "session.totalLaps",
            Self::SessionRaceState => "session.raceState",
            Self::Tyres => "tyres",
            Self::Conditions => "conditions",
            Self::Damage => "damage",
            Self::Aero => "aero",
            Self::Navigation => "navigation",
            Self::Lights => "lights",
            Self::Job => "job",
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
    /// New shift-light progression when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rev_lights: Option<Normalized>,
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
    /// New fuel capacity by mass when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fuel_capacity_kg: Option<f32>,
    /// New estimated remaining laps on fuel when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fuel_remaining_laps: Option<f32>,
    /// New fuel volume when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fuel_liters: Option<f32>,
    /// New fuel capacity by volume when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fuel_capacity_liters: Option<f32>,
    /// New estimated fuel range when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fuel_range_km: Option<f32>,
    /// New low-fuel warning state when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fuel_warning: Option<bool>,
    /// New pit-limiter state when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pit_limiter: Option<bool>,
    /// New electrical energy level when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ers_energy_j: Option<f32>,
}

/// Partial lap state emitted by an adapter.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
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
    /// New current sector when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sector: Option<u8>,
    /// New sector-one time when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sector1_time_ms: Option<u32>,
    /// New sector-two time when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sector2_time_ms: Option<u32>,
    /// New delta to the car ahead when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta_to_car_in_front_ms: Option<u32>,
    /// New delta to the leader when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta_to_race_leader_ms: Option<u32>,
    /// New current-lap distance when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distance_m: Option<f32>,
    /// New total session distance when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_distance_m: Option<f32>,
    /// New safety-car delta when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_car_delta_ms: Option<i32>,
    /// New pit status when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pit_status: Option<PitStatus>,
    /// New pit-stop count when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pit_stops: Option<u8>,
    /// New accumulated time penalties when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub penalties_seconds: Option<u8>,
    /// New warning count when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warnings: Option<u8>,
    /// New corner-cutting warning count when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub corner_cutting_warnings: Option<u8>,
    /// New unserved drive-through count when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unserved_drive_through_penalties: Option<u8>,
    /// New unserved stop-go count when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unserved_stop_go_penalties: Option<u8>,
    /// New grid position when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grid_position: Option<u8>,
    /// New driver status when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub driver_status: Option<DriverStatus>,
    /// New result status when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_status: Option<ResultStatus>,
    /// New pit-lane time when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pit_lane_time_ms: Option<u32>,
    /// New pit-stop time when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pit_stop_time_ms: Option<u32>,
    /// New pit-stop penalty-service state when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pit_stop_should_serve_penalty: Option<bool>,
}

/// Partial session state emitted by an adapter.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
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
    /// New session type when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_type: Option<String>,
    /// New track length when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub track_length_m: Option<u32>,
    /// New pit-lane speed limit when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pit_speed_limit_mps: Option<f32>,
    /// New safety-car state when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_car_status: Option<SafetyCarStatus>,
    /// New player race flag when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub race_flag: Option<RaceFlag>,
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
    /// New actual compound identifier when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual_compound: Option<u8>,
    /// New visual compound identifier when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visual_compound: Option<u8>,
    /// New fitted-set age when supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub age_laps: Option<u8>,
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
    /// Partial ambient-condition update.
    #[serde(skip_serializing_if = "ConditionsState::is_empty")]
    pub conditions: ConditionsState,
    /// Partial damage update.
    #[serde(skip_serializing_if = "DamageState::is_empty")]
    pub damage: DamageState,
    /// Partial active-aerodynamics update.
    #[serde(skip_serializing_if = "AeroState::is_empty")]
    pub aero: AeroState,
    /// Partial route-advisor update.
    #[serde(skip_serializing_if = "NavigationState::is_empty")]
    pub navigation: NavigationState,
    /// Partial exterior-light update.
    #[serde(skip_serializing_if = "LightsState::is_empty")]
    pub lights: LightsState,
    /// Partial delivery-job update.
    #[serde(skip_serializing_if = "JobState::is_empty")]
    pub job: JobState,
    /// Adapter-specific partial values.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub extensions: BTreeMap<String, Value>,
}
