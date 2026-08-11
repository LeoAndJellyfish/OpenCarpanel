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
  | "vehicle.throttle"
  | "vehicle.brake"
  | "vehicle.drs"
  | "lap.current"
  | "lap.position"
  | "lap.currentTimeMs"
  | "lap.lastTimeMs"
  | "lap.deltaToBestMs"
  | "lap.invalid"
  | "session.trackId"
  | "session.remainingTimeMs"
  | "session.totalLaps"
  | "tyres";
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
 * Microseconds elapsed from the Host's monotonic clock origin.
 *
 * This interface was referenced by `undefined`'s JSON-Schema
 * via the `definition` "MonotonicTimestamp".
 */
export type MonotonicTimestamp = number;
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
 * A finite value in the inclusive range `0.0..=1.0`.
 *
 * This interface was referenced by `undefined`'s JSON-Schema
 * via the `definition` "Normalized".
 */
export type Normalized = number;
/**
 * Reason that live telemetry is not currently advancing.
 *
 * This interface was referenced by `undefined`'s JSON-Schema
 * via the `definition` "StaleReason".
 */
export type StaleReason = "game_data_timeout" | "data_source_disconnected" | "session_changed";

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
 * Structured protocol error sent to a client.
 *
 * This interface was referenced by `undefined`'s JSON-Schema
 * via the `definition` "ErrorMessage".
 */
export interface ErrorMessage {
  code: ErrorCode & {
    [k: string]: unknown;
  };
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
  data: TelemetryEvent & {
    [k: string]: unknown;
  };
  /**
   * Reliable event sequence.
   */
  seq: number;
  [k: string]: unknown;
}
/**
 * A discrete telemetry fact that must not be treated as replaceable state.
 *
 * This interface was referenced by `undefined`'s JSON-Schema
 * via the `definition` "TelemetryEvent".
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
  occurredAt: MonotonicTimestamp & {
    [k: string]: unknown;
  };
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
   * Whether the current lap has been invalidated.
   */
  invalid?: boolean | null;
  /**
   * Previous completed-lap time in milliseconds.
   */
  lastTimeMs?: number | null;
  /**
   * Current one-based race position.
   */
  position?: number | null;
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
   * Remaining session time in milliseconds.
   */
  remainingTimeMs?: number | null;
  /**
   * Scheduled total number of laps when known.
   */
  totalLaps?: number | null;
  /**
   * Stable game-provided or mapped track identifier.
   */
  trackId?: string | null;
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
  data: TelemetrySnapshot & {
    [k: string]: unknown;
  };
  /**
   * Snapshot publication sequence.
   */
  seq: number;
  [k: string]: unknown;
}
/**
 * A complete, game-neutral view of the latest known telemetry state.
 *
 * This interface was referenced by `undefined`'s JSON-Schema
 * via the `definition` "TelemetrySnapshot".
 */
export interface TelemetrySnapshot {
  /**
   * Adapter-specific values that do not yet have stable cross-game semantics.
   */
  extensions?: {
    [k: string]: unknown;
  };
  lap?: LapState & {
    [k: string]: unknown;
  };
  meta?: Meta & {
    [k: string]: unknown;
  };
  session?: SessionState & {
    [k: string]: unknown;
  };
  tyres?: TyreState & {
    [k: string]: unknown;
  };
  vehicle?: VehicleState & {
    [k: string]: unknown;
  };
  [k: string]: unknown;
}
/**
 * State for all four tyre corners.
 *
 * This interface was referenced by `undefined`'s JSON-Schema
 * via the `definition` "TyreState".
 */
export interface TyreState {
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
 * Current player-vehicle state in standard units.
 *
 * This interface was referenced by `undefined`'s JSON-Schema
 * via the `definition` "VehicleState".
 */
export interface VehicleState {
  /**
   * Brake input in the inclusive unit interval.
   */
  brake?: Normalized | null;
  drs?: DrsState & {
    [k: string]: unknown;
  };
  /**
   * Available electrical energy in joules.
   */
  ersEnergyJ?: number | null;
  /**
   * Remaining fuel mass in kilograms.
   */
  fuelKg?: number | null;
  gear?: Gear & {
    [k: string]: unknown;
  };
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
  reason: StaleReason & {
    [k: string]: unknown;
  };
  /**
   * Host-monotonic time at which the state became stale.
   */
  sinceUs: number;
  [k: string]: unknown;
}
