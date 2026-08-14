/**
 * Generated from the committed OpenCarpanel JSON Schemas.
 * Do not edit by hand; run `npm run generate:web-types`.
 */

/**
 * Message sent from the Host to a dashboard client.
 */
export type ServerMessage = {
  /**
   * Wire protocol major version.
   */
  v: 1;
  [k: string]: unknown;
} & (
  | (ServerHello & {
      type: "hello";
      [k: string]: unknown;
    })
  | (SnapshotMessage & {
      type: "snapshot";
      [k: string]: unknown;
    })
  | (EventMessage & {
      type: "event";
      [k: string]: unknown;
    })
  | (CapabilitiesMessage & {
      type: "capabilities";
      [k: string]: unknown;
    })
  | (ResyncRequiredMessage & {
      type: "resync_required";
      [k: string]: unknown;
    })
  | (StaleMessage & {
      type: "stale";
      [k: string]: unknown;
    })
  | (ErrorMessage & {
      type: "error";
      [k: string]: unknown;
    })
);
/**
 * 2026 active-aerodynamics mode.
 *
 * This interface was referenced by `undefined`'s JSON-Schema
 * via the `definition` "ActiveAeroMode".
 */
export type ActiveAeroMode = "corner" | "straight";
/**
 * Stable fields that an adapter can advertise to clients.
 *
 * This interface was referenced by `undefined`'s JSON-Schema
 * via the `definition` "TelemetryField".
 */
export type TelemetryField =
  | "vehicle.speedMps"
  | "vehicle.gear"
  | "vehicle.rpm"
  | "vehicle.rpmMax"
  | "vehicle.revLights"
  | "vehicle.throttle"
  | "vehicle.brake"
  | "vehicle.drs"
  | "vehicle.fuel"
  | "vehicle.pitLimiter"
  | "lap.current"
  | "lap.position"
  | "lap.currentTimeMs"
  | "lap.lastTimeMs"
  | "lap.deltaToBestMs"
  | "lap.invalid"
  | "lap.raceState"
  | "session.trackId"
  | "session.remainingTimeMs"
  | "session.totalLaps"
  | "session.raceState"
  | "tyres"
  | "conditions"
  | "damage"
  | "aero"
  | "navigation"
  | "lights"
  | "job";
/**
 * Weather reported for the current session.
 *
 * This interface was referenced by `undefined`'s JSON-Schema
 * via the `definition` "WeatherCondition".
 */
export type WeatherCondition = "clear" | "light_cloud" | "overcast" | "light_rain" | "heavy_rain" | "storm";
/**
 * A finite value in the inclusive range `0.0..=1.0`.
 *
 * This interface was referenced by `undefined`'s JSON-Schema
 * via the `definition` "Normalized".
 */
export type Normalized = number;
/**
 * Player activity within the current lap.
 *
 * This interface was referenced by `undefined`'s JSON-Schema
 * via the `definition` "DriverStatus".
 */
export type DriverStatus = "in_garage" | "flying_lap" | "in_lap" | "out_lap" | "on_track";
/**
 * Driver-reduction-system state normalized across supported games.
 *
 * This interface was referenced by `undefined`'s JSON-Schema
 * via the `definition` "DrsState".
 */
export type DrsState = "unavailable" | "available" | "active" | "unknown";
/**
 * Stable machine-readable error categories.
 *
 * This interface was referenced by `undefined`'s JSON-Schema
 * via the `definition` "ErrorCode".
 */
export type ErrorCode =
  | "pairing_required"
  | "invalid_pairing_token"
  | "pairing_token_expired"
  | "unsupported_version"
  | "invalid_message"
  | "message_too_large"
  | "internal";
/**
 * Gear selected by the game.
 *
 * This interface was referenced by `undefined`'s JSON-Schema
 * via the `definition` "Gear".
 */
export type Gear =
  | "reverse"
  | "neutral"
  | {
      forward: number;
    }
  | "unknown";
/**
 * Player pit state.
 *
 * This interface was referenced by `undefined`'s JSON-Schema
 * via the `definition` "PitStatus".
 */
export type PitStatus = "none" | "pitting" | "in_pit_area";
/**
 * Classification state for the player vehicle.
 *
 * This interface was referenced by `undefined`'s JSON-Schema
 * via the `definition` "ResultStatus".
 */
export type ResultStatus =
  "invalid" | "inactive" | "active" | "finished" | "did_not_finish" | "disqualified" | "not_classified" | "retired";
/**
 * Microseconds elapsed from the Host's monotonic clock origin.
 *
 * This interface was referenced by `undefined`'s JSON-Schema
 * via the `definition` "MonotonicTimestamp".
 */
export type MonotonicTimestamp = number;
/**
 * Race-control flag applying to the player vehicle.
 *
 * This interface was referenced by `undefined`'s JSON-Schema
 * via the `definition` "RaceFlag".
 */
export type RaceFlag = "none" | "green" | "blue" | "yellow" | "red";
/**
 * Safety-car state for the current session.
 *
 * This interface was referenced by `undefined`'s JSON-Schema
 * via the `definition` "SafetyCarStatus".
 */
export type SafetyCarStatus = "none" | "full" | "virtual" | "formation_lap";
/**
 * Reason that live telemetry is not currently advancing.
 *
 * This interface was referenced by `undefined`'s JSON-Schema
 * via the `definition` "StaleReason".
 */
export type StaleReason = "game_data_timeout" | "data_source_disconnected" | "session_changed";

/**
 * 2026 active-aerodynamics and overtake state.
 *
 * This interface was referenced by `undefined`'s JSON-Schema
 * via the `definition` "AeroState".
 */
export interface AeroState {
  /**
   * Distance until active aero becomes available in metres.
   */
  activationDistanceM?: number | null;
  /**
   * Whether active aero is currently available.
   */
  available?: boolean | null;
  /**
   * Whether the game reports the car is driving the wrong way.
   */
  drivingWrongWay?: boolean | null;
  /**
   * Current active-aero mode.
   */
  mode?: ActiveAeroMode | null;
  /**
   * Distance until overtake mode becomes available in metres.
   */
  overtakeActivationDistanceM?: number | null;
  /**
   * Whether overtake mode is active.
   */
  overtakeActive?: boolean | null;
  /**
   * Whether overtake mode is available.
   */
  overtakeAvailable?: boolean | null;
  /**
   * Whether the car uses the 2026 regulations.
   */
  regulations2026?: boolean | null;
  [k: string]: unknown;
}
/**
 * Active adapter capabilities.
 *
 * This interface was referenced by `undefined`'s JSON-Schema
 * via the `definition` "CapabilitiesMessage".
 */
export interface CapabilitiesMessage {
  /**
   * Namespaced adapter extension paths.
   */
  extensions: string[];
  /**
   * Stable canonical telemetry paths.
   */
  fields: TelemetryField[];
  [k: string]: unknown;
}
/**
 * Current ambient and track conditions.
 *
 * This interface was referenced by `undefined`'s JSON-Schema
 * via the `definition` "ConditionsState".
 */
export interface ConditionsState {
  /**
   * Air temperature in degrees Celsius.
   */
  airTemperatureC?: number | null;
  /**
   * Track temperature in degrees Celsius.
   */
  trackTemperatureC?: number | null;
  /**
   * Current weather category.
   */
  weather?: WeatherCondition | null;
  [k: string]: unknown;
}
/**
 * Player-vehicle damage not naturally represented by one tyre corner.
 *
 * This interface was referenced by `undefined`'s JSON-Schema
 * via the `definition` "DamageState".
 */
export interface DamageState {
  /**
   * Diffuser damage.
   */
  diffuser?: Normalized | null;
  /**
   * DRS system fault state.
   */
  drsFault?: boolean | null;
  /**
   * Overall engine damage.
   */
  engine?: Normalized | null;
  /**
   * Whether the engine has blown.
   */
  engineBlown?: boolean | null;
  /**
   * Control-electronics wear.
   */
  engineCeWear?: Normalized | null;
  /**
   * Energy-store wear.
   */
  engineEsWear?: Normalized | null;
  /**
   * Internal-combustion-engine wear.
   */
  engineIceWear?: Normalized | null;
  /**
   * MGU-H wear.
   */
  engineMguhWear?: Normalized | null;
  /**
   * MGU-K wear.
   */
  engineMgukWear?: Normalized | null;
  /**
   * Whether the engine has seized.
   */
  engineSeized?: boolean | null;
  /**
   * Turbocharger wear.
   */
  engineTcWear?: Normalized | null;
  /**
   * ERS system fault state.
   */
  ersFault?: boolean | null;
  /**
   * Floor damage.
   */
  floor?: Normalized | null;
  /**
   * Front-left wing damage.
   */
  frontLeftWing?: Normalized | null;
  /**
   * Front-right wing damage.
   */
  frontRightWing?: Normalized | null;
  /**
   * Gearbox damage.
   */
  gearbox?: Normalized | null;
  /**
   * Rear-wing damage.
   */
  rearWing?: Normalized | null;
  /**
   * Sidepod damage.
   */
  sidepod?: Normalized | null;
  [k: string]: unknown;
}
/**
 * Structured protocol error sent to a client.
 *
 * This interface was referenced by `undefined`'s JSON-Schema
 * via the `definition` "ErrorMessage".
 */
export interface ErrorMessage {
  /**
   * Stable programmatic error code.
   */
  code:
    | "pairing_required"
    | "invalid_pairing_token"
    | "pairing_token_expired"
    | "unsupported_version"
    | "invalid_message"
    | "message_too_large"
    | "internal";
  /**
   * Sanitized human-readable explanation.
   */
  message: string;
  /**
   * Whether retrying without user intervention can succeed.
   */
  retryable: boolean;
  [k: string]: unknown;
}
/**
 * Ordered reliable event.
 *
 * This interface was referenced by `undefined`'s JSON-Schema
 * via the `definition` "EventMessage".
 */
export interface EventMessage {
  data: TelemetryEvent;
  /**
   * Reliable event sequence.
   */
  seq: number;
  [k: string]: unknown;
}
/**
 * Canonical event data.
 */
export interface TelemetryEvent {
  /**
   * Event-specific structured data.
   */
  data?: {
    [k: string]: unknown;
  };
  /**
   * Stable dotted event name, such as `lap.completed`.
   */
  name: string;
  /**
   * Microseconds elapsed from the Host's monotonic clock origin.
   */
  occurredAt: number;
  [k: string]: unknown;
}
/**
 * Active truck-delivery job.
 *
 * This interface was referenced by `undefined`'s JSON-Schema
 * via the `definition` "JobState".
 */
export interface JobState {
  /**
   * Whether a job is active.
   */
  active?: boolean | null;
  /**
   * Localized cargo name.
   */
  cargo?: string | null;
  /**
   * Whether cargo is loaded.
   */
  cargoLoaded?: boolean | null;
  /**
   * Cargo mass in kilograms.
   */
  cargoMassKg?: number | null;
  /**
   * Absolute in-game delivery deadline.
   */
  deliveryTime?: number | null;
  /**
   * Localized destination city.
   */
  destinationCity?: string | null;
  /**
   * Expected income in the game's native currency.
   */
  income?: number | null;
  /**
   * Planned distance in simulated kilometres.
   */
  plannedDistanceKm?: number | null;
  /**
   * Localized source city.
   */
  sourceCity?: string | null;
  /**
   * Whether this is a special-transport job.
   */
  special?: boolean | null;
  [k: string]: unknown;
}
/**
 * Current lap and race-position state.
 *
 * This interface was referenced by `undefined`'s JSON-Schema
 * via the `definition` "LapState".
 */
export interface LapState {
  /**
   * Accumulated corner-cutting warnings.
   */
  cornerCuttingWarnings?: number | null;
  /**
   * Current one-based lap number.
   */
  current?: number | null;
  /**
   * Elapsed current-lap time in milliseconds.
   */
  currentTimeMs?: number | null;
  /**
   * Signed delta to the relevant best lap in milliseconds.
   */
  deltaToBestMs?: number | null;
  /**
   * Delta to the car immediately ahead in milliseconds.
   */
  deltaToCarInFrontMs?: number | null;
  /**
   * Delta to the race leader in milliseconds.
   */
  deltaToRaceLeaderMs?: number | null;
  /**
   * Distance around the current lap in metres.
   */
  distanceM?: number | null;
  /**
   * Current driver activity.
   */
  driverStatus?: DriverStatus | null;
  /**
   * Starting grid position.
   */
  gridPosition?: number | null;
  /**
   * Whether the current lap has been invalidated.
   */
  invalid?: boolean | null;
  /**
   * Previous completed-lap time in milliseconds.
   */
  lastTimeMs?: number | null;
  /**
   * Accumulated time penalties in seconds.
   */
  penaltiesSeconds?: number | null;
  /**
   * Time spent in the pit lane in milliseconds.
   */
  pitLaneTimeMs?: number | null;
  /**
   * Current pit state.
   */
  pitStatus?: PitStatus | null;
  /**
   * Whether a penalty should be served during the current pit stop.
   */
  pitStopShouldServePenalty?: boolean | null;
  /**
   * Current pit-stop time in milliseconds.
   */
  pitStopTimeMs?: number | null;
  /**
   * Pit stops completed in the current race.
   */
  pitStops?: number | null;
  /**
   * Current one-based race position.
   */
  position?: number | null;
  /**
   * Current classification state.
   */
  resultStatus?: ResultStatus | null;
  /**
   * Signed safety-car delta in milliseconds.
   */
  safetyCarDeltaMs?: number | null;
  /**
   * Current one-based sector number.
   */
  sector?: number | null;
  /**
   * Completed sector-one time in milliseconds.
   */
  sector1TimeMs?: number | null;
  /**
   * Completed sector-two time in milliseconds.
   */
  sector2TimeMs?: number | null;
  /**
   * Total distance travelled in the session in metres.
   */
  totalDistanceM?: number | null;
  /**
   * Unserved drive-through penalties.
   */
  unservedDriveThroughPenalties?: number | null;
  /**
   * Unserved stop-go penalties.
   */
  unservedStopGoPenalties?: number | null;
  /**
   * Accumulated warnings.
   */
  warnings?: number | null;
  [k: string]: unknown;
}
/**
 * Exterior-light state for truck simulators.
 *
 * This interface was referenced by `undefined`'s JSON-Schema
 * via the `definition` "LightsState".
 */
export interface LightsState {
  /**
   * Beacon lights.
   */
  beacon?: boolean | null;
  /**
   * Brake lights.
   */
  brake?: boolean | null;
  /**
   * Hazard-warning state.
   */
  hazard?: boolean | null;
  /**
   * High-beam headlights.
   */
  highBeam?: boolean | null;
  /**
   * Logical left indicator state.
   */
  leftIndicator?: boolean | null;
  /**
   * Low-beam headlights.
   */
  lowBeam?: boolean | null;
  /**
   * Parking lights.
   */
  parking?: boolean | null;
  /**
   * Reverse lights.
   */
  reverse?: boolean | null;
  /**
   * Logical right indicator state.
   */
  rightIndicator?: boolean | null;
  [k: string]: unknown;
}
/**
 * Metadata attached to every complete telemetry snapshot.
 *
 * This interface was referenced by `undefined`'s JSON-Schema
 * via the `definition` "Meta".
 */
export interface Meta {
  /**
   * Time at which the newest contributing datagram reached the Host.
   */
  capturedAt?: MonotonicTimestamp | null;
  /**
   * Stable adapter identifier, such as `f1-24`.
   */
  gameId?: string | null;
  /**
   * Schema version used by this snapshot.
   */
  schemaVersion?: number;
  /**
   * Monotonic snapshot sequence assigned by the core.
   */
  sequence?: number;
  /**
   * Opaque identifier for the active game session.
   */
  sessionId?: string | null;
  [k: string]: unknown;
}
/**
 * Route-advisor state for truck simulators.
 *
 * This interface was referenced by `undefined`'s JSON-Schema
 * via the `definition` "NavigationState".
 */
export interface NavigationState {
  /**
   * Remaining route distance in metres.
   */
  distanceM?: number | null;
  /**
   * Current navigation speed limit in metres per second.
   */
  speedLimitMps?: number | null;
  /**
   * Estimated route time in seconds.
   */
  timeS?: number | null;
  [k: string]: unknown;
}
/**
 * Reliable-event history can no longer satisfy the requested sequence.
 *
 * This interface was referenced by `undefined`'s JSON-Schema
 * via the `definition` "ResyncRequiredMessage".
 */
export interface ResyncRequiredMessage {
  /**
   * Newest event present when this message was built.
   */
  newestEventSeq: number;
  /**
   * Oldest event still present in the bounded buffer.
   */
  oldestAvailableEventSeq: number;
  [k: string]: unknown;
}
/**
 * Host response to a successful hello.
 *
 * This interface was referenced by `undefined`'s JSON-Schema
 * via the `definition` "ServerHello".
 */
export interface ServerHello {
  /**
   * Device session issued after one-time pairing.
   */
  deviceSession?: string | null;
  /**
   * Protocol version selected by the Host.
   */
  protocolVersion: number;
  /**
   * Host application version.
   */
  serverVersion: string;
  [k: string]: unknown;
}
/**
 * Current session state that is meaningful across games.
 *
 * This interface was referenced by `undefined`'s JSON-Schema
 * via the `definition` "SessionState".
 */
export interface SessionState {
  /**
   * Pit-lane speed limit in metres per second.
   */
  pitSpeedLimitMps?: number | null;
  /**
   * Current FIA or race-control flag for the player vehicle.
   */
  raceFlag?: RaceFlag | null;
  /**
   * Remaining session time in milliseconds.
   */
  remainingTimeMs?: number | null;
  /**
   * Current safety-car state.
   */
  safetyCarStatus?: SafetyCarStatus | null;
  /**
   * Human-readable stable session type, such as `race` or `qualifying_1`.
   */
  sessionType?: string | null;
  /**
   * Scheduled total number of laps when known.
   */
  totalLaps?: number | null;
  /**
   * Stable game-provided or mapped track identifier.
   */
  trackId?: string | null;
  /**
   * Track length in metres.
   */
  trackLengthM?: number | null;
  [k: string]: unknown;
}
/**
 * Replaceable latest telemetry state.
 *
 * This interface was referenced by `undefined`'s JSON-Schema
 * via the `definition` "SnapshotMessage".
 */
export interface SnapshotMessage {
  /**
   * Host-monotonic capture time in microseconds.
   */
  capturedAtUs: number;
  data: TelemetrySnapshot;
  /**
   * Snapshot publication sequence.
   */
  seq: number;
  [k: string]: unknown;
}
/**
 * Complete canonical telemetry state.
 */
export interface TelemetrySnapshot {
  aero?: AeroState1;
  conditions?: ConditionsState1;
  damage?: DamageState1;
  /**
   * Adapter-specific values that do not yet have stable cross-game semantics.
   */
  extensions?: {
    [k: string]: unknown;
  };
  job?: JobState1;
  lap?: LapState1;
  lights?: LightsState1;
  meta?: Meta1;
  navigation?: NavigationState1;
  session?: SessionState1;
  tyres?: TyreState;
  vehicle?: VehicleState;
  [k: string]: unknown;
}
/**
 * Active-aerodynamics state.
 */
export interface AeroState1 {
  /**
   * Distance until active aero becomes available in metres.
   */
  activationDistanceM?: number | null;
  /**
   * Whether active aero is currently available.
   */
  available?: boolean | null;
  /**
   * Whether the game reports the car is driving the wrong way.
   */
  drivingWrongWay?: boolean | null;
  /**
   * Current active-aero mode.
   */
  mode?: ActiveAeroMode | null;
  /**
   * Distance until overtake mode becomes available in metres.
   */
  overtakeActivationDistanceM?: number | null;
  /**
   * Whether overtake mode is active.
   */
  overtakeActive?: boolean | null;
  /**
   * Whether overtake mode is available.
   */
  overtakeAvailable?: boolean | null;
  /**
   * Whether the car uses the 2026 regulations.
   */
  regulations2026?: boolean | null;
  [k: string]: unknown;
}
/**
 * Ambient and track conditions.
 */
export interface ConditionsState1 {
  /**
   * Air temperature in degrees Celsius.
   */
  airTemperatureC?: number | null;
  /**
   * Track temperature in degrees Celsius.
   */
  trackTemperatureC?: number | null;
  /**
   * Current weather category.
   */
  weather?: WeatherCondition | null;
  [k: string]: unknown;
}
/**
 * Player-vehicle damage.
 */
export interface DamageState1 {
  /**
   * Diffuser damage.
   */
  diffuser?: Normalized | null;
  /**
   * DRS system fault state.
   */
  drsFault?: boolean | null;
  /**
   * Overall engine damage.
   */
  engine?: Normalized | null;
  /**
   * Whether the engine has blown.
   */
  engineBlown?: boolean | null;
  /**
   * Control-electronics wear.
   */
  engineCeWear?: Normalized | null;
  /**
   * Energy-store wear.
   */
  engineEsWear?: Normalized | null;
  /**
   * Internal-combustion-engine wear.
   */
  engineIceWear?: Normalized | null;
  /**
   * MGU-H wear.
   */
  engineMguhWear?: Normalized | null;
  /**
   * MGU-K wear.
   */
  engineMgukWear?: Normalized | null;
  /**
   * Whether the engine has seized.
   */
  engineSeized?: boolean | null;
  /**
   * Turbocharger wear.
   */
  engineTcWear?: Normalized | null;
  /**
   * ERS system fault state.
   */
  ersFault?: boolean | null;
  /**
   * Floor damage.
   */
  floor?: Normalized | null;
  /**
   * Front-left wing damage.
   */
  frontLeftWing?: Normalized | null;
  /**
   * Front-right wing damage.
   */
  frontRightWing?: Normalized | null;
  /**
   * Gearbox damage.
   */
  gearbox?: Normalized | null;
  /**
   * Rear-wing damage.
   */
  rearWing?: Normalized | null;
  /**
   * Sidepod damage.
   */
  sidepod?: Normalized | null;
  [k: string]: unknown;
}
/**
 * Active delivery job.
 */
export interface JobState1 {
  /**
   * Whether a job is active.
   */
  active?: boolean | null;
  /**
   * Localized cargo name.
   */
  cargo?: string | null;
  /**
   * Whether cargo is loaded.
   */
  cargoLoaded?: boolean | null;
  /**
   * Cargo mass in kilograms.
   */
  cargoMassKg?: number | null;
  /**
   * Absolute in-game delivery deadline.
   */
  deliveryTime?: number | null;
  /**
   * Localized destination city.
   */
  destinationCity?: string | null;
  /**
   * Expected income in the game's native currency.
   */
  income?: number | null;
  /**
   * Planned distance in simulated kilometres.
   */
  plannedDistanceKm?: number | null;
  /**
   * Localized source city.
   */
  sourceCity?: string | null;
  /**
   * Whether this is a special-transport job.
   */
  special?: boolean | null;
  [k: string]: unknown;
}
/**
 * Lap and race-position state.
 */
export interface LapState1 {
  /**
   * Accumulated corner-cutting warnings.
   */
  cornerCuttingWarnings?: number | null;
  /**
   * Current one-based lap number.
   */
  current?: number | null;
  /**
   * Elapsed current-lap time in milliseconds.
   */
  currentTimeMs?: number | null;
  /**
   * Signed delta to the relevant best lap in milliseconds.
   */
  deltaToBestMs?: number | null;
  /**
   * Delta to the car immediately ahead in milliseconds.
   */
  deltaToCarInFrontMs?: number | null;
  /**
   * Delta to the race leader in milliseconds.
   */
  deltaToRaceLeaderMs?: number | null;
  /**
   * Distance around the current lap in metres.
   */
  distanceM?: number | null;
  /**
   * Current driver activity.
   */
  driverStatus?: DriverStatus | null;
  /**
   * Starting grid position.
   */
  gridPosition?: number | null;
  /**
   * Whether the current lap has been invalidated.
   */
  invalid?: boolean | null;
  /**
   * Previous completed-lap time in milliseconds.
   */
  lastTimeMs?: number | null;
  /**
   * Accumulated time penalties in seconds.
   */
  penaltiesSeconds?: number | null;
  /**
   * Time spent in the pit lane in milliseconds.
   */
  pitLaneTimeMs?: number | null;
  /**
   * Current pit state.
   */
  pitStatus?: PitStatus | null;
  /**
   * Whether a penalty should be served during the current pit stop.
   */
  pitStopShouldServePenalty?: boolean | null;
  /**
   * Current pit-stop time in milliseconds.
   */
  pitStopTimeMs?: number | null;
  /**
   * Pit stops completed in the current race.
   */
  pitStops?: number | null;
  /**
   * Current one-based race position.
   */
  position?: number | null;
  /**
   * Current classification state.
   */
  resultStatus?: ResultStatus | null;
  /**
   * Signed safety-car delta in milliseconds.
   */
  safetyCarDeltaMs?: number | null;
  /**
   * Current one-based sector number.
   */
  sector?: number | null;
  /**
   * Completed sector-one time in milliseconds.
   */
  sector1TimeMs?: number | null;
  /**
   * Completed sector-two time in milliseconds.
   */
  sector2TimeMs?: number | null;
  /**
   * Total distance travelled in the session in metres.
   */
  totalDistanceM?: number | null;
  /**
   * Unserved drive-through penalties.
   */
  unservedDriveThroughPenalties?: number | null;
  /**
   * Unserved stop-go penalties.
   */
  unservedStopGoPenalties?: number | null;
  /**
   * Accumulated warnings.
   */
  warnings?: number | null;
  [k: string]: unknown;
}
/**
 * Exterior-light state.
 */
export interface LightsState1 {
  /**
   * Beacon lights.
   */
  beacon?: boolean | null;
  /**
   * Brake lights.
   */
  brake?: boolean | null;
  /**
   * Hazard-warning state.
   */
  hazard?: boolean | null;
  /**
   * High-beam headlights.
   */
  highBeam?: boolean | null;
  /**
   * Logical left indicator state.
   */
  leftIndicator?: boolean | null;
  /**
   * Low-beam headlights.
   */
  lowBeam?: boolean | null;
  /**
   * Parking lights.
   */
  parking?: boolean | null;
  /**
   * Reverse lights.
   */
  reverse?: boolean | null;
  /**
   * Logical right indicator state.
   */
  rightIndicator?: boolean | null;
  [k: string]: unknown;
}
/**
 * Snapshot metadata.
 */
export interface Meta1 {
  /**
   * Time at which the newest contributing datagram reached the Host.
   */
  capturedAt?: MonotonicTimestamp | null;
  /**
   * Stable adapter identifier, such as `f1-24`.
   */
  gameId?: string | null;
  /**
   * Schema version used by this snapshot.
   */
  schemaVersion?: number;
  /**
   * Monotonic snapshot sequence assigned by the core.
   */
  sequence?: number;
  /**
   * Opaque identifier for the active game session.
   */
  sessionId?: string | null;
  [k: string]: unknown;
}
/**
 * Route-advisor state.
 */
export interface NavigationState1 {
  /**
   * Remaining route distance in metres.
   */
  distanceM?: number | null;
  /**
   * Current navigation speed limit in metres per second.
   */
  speedLimitMps?: number | null;
  /**
   * Estimated route time in seconds.
   */
  timeS?: number | null;
  [k: string]: unknown;
}
/**
 * Session state.
 */
export interface SessionState1 {
  /**
   * Pit-lane speed limit in metres per second.
   */
  pitSpeedLimitMps?: number | null;
  /**
   * Current FIA or race-control flag for the player vehicle.
   */
  raceFlag?: RaceFlag | null;
  /**
   * Remaining session time in milliseconds.
   */
  remainingTimeMs?: number | null;
  /**
   * Current safety-car state.
   */
  safetyCarStatus?: SafetyCarStatus | null;
  /**
   * Human-readable stable session type, such as `race` or `qualifying_1`.
   */
  sessionType?: string | null;
  /**
   * Scheduled total number of laps when known.
   */
  totalLaps?: number | null;
  /**
   * Stable game-provided or mapped track identifier.
   */
  trackId?: string | null;
  /**
   * Track length in metres.
   */
  trackLengthM?: number | null;
  [k: string]: unknown;
}
/**
 * Four-corner tyre state.
 */
export interface TyreState {
  /**
   * Game-defined actual compound identifier.
   */
  actualCompound?: number | null;
  /**
   * Age of the fitted tyre set in laps.
   */
  ageLaps?: number | null;
  /**
   * Front-left tyre.
   */
  frontLeft?: TyreCornerState | null;
  /**
   * Front-right tyre.
   */
  frontRight?: TyreCornerState | null;
  /**
   * Rear-left tyre.
   */
  rearLeft?: TyreCornerState | null;
  /**
   * Rear-right tyre.
   */
  rearRight?: TyreCornerState | null;
  /**
   * Game-defined visual compound identifier.
   */
  visualCompound?: number | null;
  [k: string]: unknown;
}
/**
 * State for one tyre corner.
 *
 * This interface was referenced by `undefined`'s JSON-Schema
 * via the `definition` "TyreCornerState".
 */
export interface TyreCornerState {
  /**
   * Tyre blistering in the inclusive unit interval.
   */
  blister?: Normalized | null;
  /**
   * Brake damage at this corner in the inclusive unit interval.
   */
  brakeDamage?: Normalized | null;
  /**
   * Tyre damage in the inclusive unit interval.
   */
  damage?: Normalized | null;
  /**
   * Tyre inner temperature in degrees Celsius.
   */
  innerTemperatureC?: number | null;
  /**
   * Tyre pressure in pascals.
   */
  pressurePa?: number | null;
  /**
   * Tyre surface temperature in degrees Celsius.
   */
  surfaceTemperatureC?: number | null;
  /**
   * Tyre wear in the inclusive unit interval.
   */
  wear?: Normalized | null;
  [k: string]: unknown;
}
/**
 * Player-vehicle state.
 */
export interface VehicleState {
  /**
   * Brake input in the inclusive unit interval.
   */
  brake?: Normalized | null;
  /**
   * Driver-reduction-system state.
   */
  drs?: "unavailable" | "available" | "active" | "unknown";
  /**
   * Available electrical energy in joules.
   */
  ersEnergyJ?: number | null;
  /**
   * Fuel-tank capacity in kilograms when the game reports fuel by mass.
   */
  fuelCapacityKg?: number | null;
  /**
   * Fuel-tank capacity in litres.
   */
  fuelCapacityLiters?: number | null;
  /**
   * Remaining fuel mass in kilograms.
   */
  fuelKg?: number | null;
  /**
   * Remaining fuel volume in litres.
   */
  fuelLiters?: number | null;
  /**
   * Estimated remaining fuel range in kilometres.
   */
  fuelRangeKm?: number | null;
  /**
   * Estimated laps remaining on the current fuel load.
   */
  fuelRemainingLaps?: number | null;
  /**
   * Whether the game is showing a low-fuel warning.
   */
  fuelWarning?: boolean | null;
  /**
   * Selected gear.
   */
  gear?:
    | "reverse"
    | "neutral"
    | {
        forward: number;
      }
    | "unknown";
  /**
   * Whether the pit-lane speed limiter is active.
   */
  pitLimiter?: boolean | null;
  /**
   * Game-provided shift-light progression in the inclusive unit interval.
   */
  revLights?: Normalized | null;
  /**
   * Current engine revolutions per minute.
   */
  rpm?: number | null;
  /**
   * Maximum engine revolutions per minute used for display scaling.
   */
  rpmMax?: number | null;
  /**
   * Vehicle speed in metres per second.
   */
  speedMps?: number | null;
  /**
   * Accelerator input in the inclusive unit interval.
   */
  throttle?: Normalized | null;
  [k: string]: unknown;
}
/**
 * Explicit stale-state notification.
 *
 * This interface was referenced by `undefined`'s JSON-Schema
 * via the `definition` "StaleMessage".
 */
export interface StaleMessage {
  /**
   * Why the state is stale.
   */
  reason: "game_data_timeout" | "data_source_disconnected" | "session_changed";
  /**
   * Host-monotonic time at which the state became stale.
   */
  sinceUs: number;
  [k: string]: unknown;
}
/**
 * A discrete telemetry fact that must not be treated as replaceable state.
 *
 * This interface was referenced by `undefined`'s JSON-Schema
 * via the `definition` "TelemetryEvent".
 */
export interface TelemetryEvent1 {
  /**
   * Event-specific structured data.
   */
  data?: {
    [k: string]: unknown;
  };
  /**
   * Stable dotted event name, such as `lap.completed`.
   */
  name: string;
  /**
   * Microseconds elapsed from the Host's monotonic clock origin.
   */
  occurredAt: number;
  [k: string]: unknown;
}
/**
 * A complete, game-neutral view of the latest known telemetry state.
 *
 * This interface was referenced by `undefined`'s JSON-Schema
 * via the `definition` "TelemetrySnapshot".
 */
export interface TelemetrySnapshot1 {
  aero?: AeroState1;
  conditions?: ConditionsState1;
  damage?: DamageState1;
  /**
   * Adapter-specific values that do not yet have stable cross-game semantics.
   */
  extensions?: {
    [k: string]: unknown;
  };
  job?: JobState1;
  lap?: LapState1;
  lights?: LightsState1;
  meta?: Meta1;
  navigation?: NavigationState1;
  session?: SessionState1;
  tyres?: TyreState;
  vehicle?: VehicleState;
  [k: string]: unknown;
}
/**
 * State for all four tyre corners.
 *
 * This interface was referenced by `undefined`'s JSON-Schema
 * via the `definition` "TyreState".
 */
export interface TyreState1 {
  /**
   * Game-defined actual compound identifier.
   */
  actualCompound?: number | null;
  /**
   * Age of the fitted tyre set in laps.
   */
  ageLaps?: number | null;
  /**
   * Front-left tyre.
   */
  frontLeft?: TyreCornerState | null;
  /**
   * Front-right tyre.
   */
  frontRight?: TyreCornerState | null;
  /**
   * Rear-left tyre.
   */
  rearLeft?: TyreCornerState | null;
  /**
   * Rear-right tyre.
   */
  rearRight?: TyreCornerState | null;
  /**
   * Game-defined visual compound identifier.
   */
  visualCompound?: number | null;
  [k: string]: unknown;
}
/**
 * Current player-vehicle state in standard units.
 *
 * This interface was referenced by `undefined`'s JSON-Schema
 * via the `definition` "VehicleState".
 */
export interface VehicleState1 {
  /**
   * Brake input in the inclusive unit interval.
   */
  brake?: Normalized | null;
  /**
   * Driver-reduction-system state.
   */
  drs?: "unavailable" | "available" | "active" | "unknown";
  /**
   * Available electrical energy in joules.
   */
  ersEnergyJ?: number | null;
  /**
   * Fuel-tank capacity in kilograms when the game reports fuel by mass.
   */
  fuelCapacityKg?: number | null;
  /**
   * Fuel-tank capacity in litres.
   */
  fuelCapacityLiters?: number | null;
  /**
   * Remaining fuel mass in kilograms.
   */
  fuelKg?: number | null;
  /**
   * Remaining fuel volume in litres.
   */
  fuelLiters?: number | null;
  /**
   * Estimated remaining fuel range in kilometres.
   */
  fuelRangeKm?: number | null;
  /**
   * Estimated laps remaining on the current fuel load.
   */
  fuelRemainingLaps?: number | null;
  /**
   * Whether the game is showing a low-fuel warning.
   */
  fuelWarning?: boolean | null;
  /**
   * Selected gear.
   */
  gear?:
    | "reverse"
    | "neutral"
    | {
        forward: number;
      }
    | "unknown";
  /**
   * Whether the pit-lane speed limiter is active.
   */
  pitLimiter?: boolean | null;
  /**
   * Game-provided shift-light progression in the inclusive unit interval.
   */
  revLights?: Normalized | null;
  /**
   * Current engine revolutions per minute.
   */
  rpm?: number | null;
  /**
   * Maximum engine revolutions per minute used for display scaling.
   */
  rpmMax?: number | null;
  /**
   * Vehicle speed in metres per second.
   */
  speedMps?: number | null;
  /**
   * Accelerator input in the inclusive unit interval.
   */
  throttle?: Normalized | null;
  [k: string]: unknown;
}
