import type {
  DrsState,
  Gear,
  SnapshotMessage,
  TelemetrySnapshot,
} from "@opencarpanel/widget-sdk";

import { interpolateLinear, type LinearInterpolation } from "./interpolate";

export const ALL_DASHBOARD_FIELDS = [
  "vehicle.speedMps",
  "vehicle.rpm",
  "vehicle.rpmMax",
  "vehicle.revLights",
  "vehicle.throttle",
  "vehicle.brake",
  "vehicle.gear",
  "vehicle.drs",
  "vehicle.fuel",
  "lap",
  "session",
  "tyres",
  "conditions",
  "damage",
  "aero",
  "navigation",
  "lights",
  "job",
  "meta.gameId",
  "meta.sessionId",
  "system.stale",
] as const;

export interface FuelState {
  readonly capacityKg: number | undefined;
  readonly capacityLiters: number | undefined;
  readonly kg: number | undefined;
  readonly liters: number | undefined;
  readonly rangeKm: number | undefined;
  readonly remainingLaps: number | undefined;
  readonly warning: boolean | undefined;
}

type LapState = NonNullable<TelemetrySnapshot["lap"]>;
type SessionState = NonNullable<TelemetrySnapshot["session"]>;
type TyreState = NonNullable<TelemetrySnapshot["tyres"]>;
type ConditionsState = NonNullable<TelemetrySnapshot["conditions"]>;
type DamageState = NonNullable<TelemetrySnapshot["damage"]>;
type AeroState = NonNullable<TelemetrySnapshot["aero"]>;
type NavigationState = NonNullable<TelemetrySnapshot["navigation"]>;
type LightsState = NonNullable<TelemetrySnapshot["lights"]>;
type JobState = NonNullable<TelemetrySnapshot["job"]>;

export interface TelemetryValueMap {
  "vehicle.speedMps": number | undefined;
  "vehicle.rpm": number | undefined;
  "vehicle.rpmMax": number | undefined;
  "vehicle.revLights": number | undefined;
  "vehicle.throttle": number | undefined;
  "vehicle.brake": number | undefined;
  "vehicle.gear": Gear | undefined;
  "vehicle.drs": DrsState | undefined;
  "vehicle.fuel": FuelState | undefined;
  lap: LapState | undefined;
  session: SessionState | undefined;
  tyres: TyreState | undefined;
  conditions: ConditionsState | undefined;
  damage: DamageState | undefined;
  aero: AeroState | undefined;
  navigation: NavigationState | undefined;
  lights: LightsState | undefined;
  job: JobState | undefined;
  "meta.gameId": string | undefined;
  "meta.sessionId": string | undefined;
  "system.stale": boolean;
}

export type DashboardField = keyof TelemetryValueMap;
export type TelemetryListener = (changed: ReadonlySet<DashboardField>) => void;

const CONTINUOUS_FIELDS = [
  "vehicle.speedMps",
  "vehicle.rpm",
  "vehicle.rpmMax",
  "vehicle.revLights",
  "vehicle.throttle",
] as const satisfies readonly DashboardField[];
type ContinuousField = (typeof CONTINUOUS_FIELDS)[number];

interface Subscription {
  readonly fields: ReadonlySet<DashboardField>;
  readonly listener: TelemetryListener;
}

export interface TelemetryStoreOptions {
  readonly expectedSampleIntervalMs?: number;
  readonly reducedMotion?: boolean;
}

export class TelemetryStore {
  readonly #expectedSampleIntervalMs: number;
  readonly #targets = new Map<DashboardField, TelemetryValueMap[DashboardField]>();
  readonly #interpolations = new Map<ContinuousField, LinearInterpolation>();
  readonly #subscriptions = new Map<number, Subscription>();
  #nextSubscriptionId = 1;
  #initialized = false;
  #reducedMotion: boolean;

  constructor(options: TelemetryStoreOptions = {}) {
    this.#expectedSampleIntervalMs = Math.max(1, options.expectedSampleIntervalMs ?? 1000 / 60);
    this.#reducedMotion = options.reducedMotion ?? false;
    this.#targets.set("system.stale", true);
  }

  ingest(snapshot: SnapshotMessage, receivedAtMs: number): ReadonlySet<DashboardField> {
    const values = snapshotValues(snapshot.data);
    const previousGame = this.#targets.get("meta.gameId");
    const nextGame = values["meta.gameId"];
    const previousSession = this.#targets.get("meta.sessionId");
    const nextSession = values["meta.sessionId"];
    const contextChanged =
      this.#initialized &&
      ((previousGame !== nextGame &&
        (previousGame !== undefined || nextGame !== undefined)) ||
        (previousSession !== nextSession &&
          (previousSession !== undefined || nextSession !== undefined)));
    const changed = new Set<DashboardField>();

    if (contextChanged) {
      this.#interpolations.clear();
      for (const field of ALL_DASHBOARD_FIELDS) {
        changed.add(field);
      }
    }

    for (const field of CONTINUOUS_FIELDS) {
      this.#updateContinuous(field, values[field], receivedAtMs, contextChanged, changed);
    }
    this.#updateDiscrete("vehicle.brake", values["vehicle.brake"], changed);
    this.#updateDiscrete("vehicle.gear", values["vehicle.gear"], changed);
    this.#updateDiscrete("vehicle.drs", values["vehicle.drs"], changed);
    this.#updateDiscrete("vehicle.fuel", values["vehicle.fuel"], changed);
    this.#updateDiscrete("lap", values.lap, changed);
    this.#updateDiscrete("session", values.session, changed);
    this.#updateDiscrete("tyres", values.tyres, changed);
    this.#updateDiscrete("conditions", values.conditions, changed);
    this.#updateDiscrete("damage", values.damage, changed);
    this.#updateDiscrete("aero", values.aero, changed);
    this.#updateDiscrete("navigation", values.navigation, changed);
    this.#updateDiscrete("lights", values.lights, changed);
    this.#updateDiscrete("job", values.job, changed);
    this.#updateDiscrete("meta.gameId", nextGame, changed);
    this.#updateDiscrete("meta.sessionId", nextSession, changed);
    this.#updateDiscrete("system.stale", values["system.stale"], changed);

    this.#initialized = true;
    this.#notify(changed);
    return changed;
  }

  read<Field extends DashboardField>(field: Field, nowMs: number): TelemetryValueMap[Field] {
    if (isContinuousField(field)) {
      const interpolation = this.#interpolations.get(field);
      if (interpolation) {
        return interpolateLinear(interpolation, nowMs) as TelemetryValueMap[Field];
      }
    }
    return this.#targets.get(field) as TelemetryValueMap[Field];
  }

  activeContinuousFields(nowMs: number): ReadonlySet<DashboardField> {
    const active = new Set<DashboardField>();
    for (const [field, interpolation] of this.#interpolations) {
      if (
        interpolation.from !== interpolation.to &&
        nowMs < interpolation.startedAtMs + interpolation.durationMs
      ) {
        active.add(field);
      }
    }
    return active;
  }

  setStale(stale: boolean): void {
    const changed = new Set<DashboardField>();
    this.#updateDiscrete("system.stale", stale, changed);
    this.#notify(changed);
  }

  setReducedMotion(reducedMotion: boolean, nowMs: number): void {
    if (this.#reducedMotion === reducedMotion) {
      return;
    }
    this.#reducedMotion = reducedMotion;
    if (reducedMotion) {
      this.resetInterpolation(nowMs);
    }
  }

  resetInterpolation(nowMs: number): void {
    const changed = new Set<DashboardField>();
    for (const field of CONTINUOUS_FIELDS) {
      const target = this.#targets.get(field);
      if (typeof target === "number") {
        this.#interpolations.set(field, {
          from: target,
          to: target,
          startedAtMs: nowMs,
          durationMs: 0,
        });
        changed.add(field);
      }
    }
    this.#notify(changed);
  }

  subscribe(fields: readonly DashboardField[], listener: TelemetryListener): () => void {
    const id = this.#nextSubscriptionId;
    this.#nextSubscriptionId += 1;
    this.#subscriptions.set(id, { fields: new Set(fields), listener });
    return () => {
      this.#subscriptions.delete(id);
    };
  }

  #updateContinuous(
    field: ContinuousField,
    next: number | undefined,
    receivedAtMs: number,
    reset: boolean,
    changed: Set<DashboardField>,
  ): void {
    const previousTarget = this.#targets.get(field);
    if (!reset && Object.is(previousTarget, next)) {
      return;
    }

    const displayed = this.read(field, receivedAtMs);
    this.#targets.set(field, next);
    if (next === undefined) {
      this.#interpolations.delete(field);
    } else {
      const from = typeof displayed === "number" ? displayed : next;
      const durationMs =
        reset || this.#reducedMotion || previousTarget === undefined
          ? 0
          : this.#expectedSampleIntervalMs;
      this.#interpolations.set(field, {
        from,
        to: next,
        startedAtMs: receivedAtMs,
        durationMs,
      });
    }
    changed.add(field);
  }

  #updateDiscrete<Field extends Exclude<DashboardField, ContinuousField>>(
    field: Field,
    next: TelemetryValueMap[Field],
    changed: Set<DashboardField>,
  ): void {
    const previous = this.#targets.get(field);
    if (sameValue(previous, next)) {
      return;
    }
    this.#targets.set(field, next);
    changed.add(field);
  }

  #notify(changed: ReadonlySet<DashboardField>): void {
    if (changed.size === 0) {
      return;
    }
    for (const subscription of this.#subscriptions.values()) {
      if (setsIntersect(subscription.fields, changed)) {
        subscription.listener(changed);
      }
    }
  }
}

function snapshotValues(snapshot: TelemetrySnapshot): TelemetryValueMap {
  const vehicle = snapshot.vehicle;
  return {
    "vehicle.speedMps": finiteNumber(vehicle?.speedMps),
    "vehicle.rpm": finiteNumber(vehicle?.rpm),
    "vehicle.rpmMax": finiteNumber(vehicle?.rpmMax),
    "vehicle.revLights": finiteNumber(vehicle?.revLights),
    "vehicle.throttle": finiteNumber(vehicle?.throttle),
    "vehicle.brake": finiteNumber(vehicle?.brake),
    "vehicle.gear": validGear(vehicle?.gear),
    "vehicle.drs": validDrs(vehicle?.drs),
    "vehicle.fuel": fuelState(vehicle),
    "lap": objectState(snapshot.lap),
    "session": objectState(snapshot.session),
    "tyres": objectState(snapshot.tyres),
    "conditions": objectState(snapshot.conditions),
    "damage": objectState(snapshot.damage),
    "aero": objectState(snapshot.aero),
    "navigation": objectState(snapshot.navigation),
    "lights": objectState(snapshot.lights),
    "job": objectState(snapshot.job),
    "meta.gameId":
      typeof snapshot.meta?.gameId === "string" ? snapshot.meta.gameId : undefined,
    "meta.sessionId": typeof snapshot.meta?.sessionId === "string" ? snapshot.meta.sessionId : undefined,
    "system.stale": finiteNumber(snapshot.meta?.capturedAt) === undefined,
  };
}

function fuelState(vehicle: TelemetrySnapshot["vehicle"]): FuelState | undefined {
  if (!vehicle) {
    return undefined;
  }
  const state: FuelState = {
    capacityKg: finiteNumber(vehicle.fuelCapacityKg),
    capacityLiters: finiteNumber(vehicle.fuelCapacityLiters),
    kg: finiteNumber(vehicle.fuelKg),
    liters: finiteNumber(vehicle.fuelLiters),
    rangeKm: finiteNumber(vehicle.fuelRangeKm),
    remainingLaps: finiteNumber(vehicle.fuelRemainingLaps),
    warning: typeof vehicle.fuelWarning === "boolean" ? vehicle.fuelWarning : undefined,
  };
  return Object.values(state).some((value) => value !== undefined) ? state : undefined;
}

function objectState<State extends object>(value: State | null | undefined): State | undefined {
  return value && Object.keys(value).length > 0 ? value : undefined;
}

function finiteNumber(value: unknown): number | undefined {
  return typeof value === "number" && Number.isFinite(value) ? value : undefined;
}

function validGear(value: unknown): Gear | undefined {
  if (value === "reverse" || value === "neutral" || value === "unknown") {
    return value;
  }
  if (value && typeof value === "object" && "forward" in value) {
    const forward = (value as { forward?: unknown }).forward;
    if (typeof forward === "number" && Number.isInteger(forward) && forward > 0) {
      return { forward };
    }
  }
  return undefined;
}

function validDrs(value: unknown): DrsState | undefined {
  return value === "unavailable" ||
    value === "available" ||
    value === "active" ||
    value === "unknown"
    ? value
    : undefined;
}

function isContinuousField(field: DashboardField): field is ContinuousField {
  return (CONTINUOUS_FIELDS as readonly DashboardField[]).includes(field);
}

function sameValue(left: unknown, right: unknown): boolean {
  if (Object.is(left, right)) {
    return true;
  }
  if (
    left &&
    right &&
    typeof left === "object" &&
    typeof right === "object" &&
    "forward" in left &&
    "forward" in right
  ) {
    return left.forward === right.forward;
  }
  return false;
}

function setsIntersect(
  left: ReadonlySet<DashboardField>,
  right: ReadonlySet<DashboardField>,
): boolean {
  for (const field of left) {
    if (right.has(field)) {
      return true;
    }
  }
  return false;
}
