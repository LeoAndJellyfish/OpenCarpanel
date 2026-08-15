import type { Gear, SnapshotMessage } from "@opensimdash/widget-sdk";
import { describe, expect, it, vi } from "vitest";

import { TelemetryStore } from "./store";

interface SnapshotValues {
  readonly gameId?: string;
  readonly sessionId?: string;
  readonly speedMps?: number;
  readonly rpm?: number;
  readonly rpmMax?: number;
  readonly gear?: Gear;
  readonly brake?: number;
}

function snapshot(sequence: number, values: SnapshotValues): SnapshotMessage {
  return {
    seq: sequence,
    capturedAtUs: sequence * 1_000,
    data: {
      meta: {
        schemaVersion: 1,
        sequence,
        ...(values.gameId === undefined ? {} : { gameId: values.gameId }),
        ...(values.sessionId === undefined ? {} : { sessionId: values.sessionId }),
      },
      vehicle: {
        ...(values.speedMps === undefined ? {} : { speedMps: values.speedMps }),
        ...(values.rpm === undefined ? {} : { rpm: values.rpm }),
        ...(values.rpmMax === undefined ? {} : { rpmMax: values.rpmMax }),
        ...(values.gear === undefined ? {} : { gear: values.gear }),
        ...(values.brake === undefined ? {} : { brake: values.brake }),
      },
    },
  };
}

describe("TelemetryStore", () => {
  it("interpolates continuous fields but applies discrete fields immediately", () => {
    const store = new TelemetryStore({ expectedSampleIntervalMs: 20 });
    store.ingest(
      snapshot(1, {
        sessionId: "session-a",
        speedMps: 10,
        rpm: 1_000,
        rpmMax: 10_000,
        gear: "neutral",
        brake: 0,
      }),
      100,
    );
    store.ingest(
      snapshot(2, {
        sessionId: "session-a",
        speedMps: 30,
        rpm: 5_000,
        rpmMax: 10_000,
        gear: { forward: 3 },
        brake: 1,
      }),
      110,
    );

    expect(store.read("vehicle.rpm", 120)).toBe(3_000);
    expect(store.read("vehicle.rpm", 130)).toBe(5_000);
    expect(store.read("vehicle.gear", 110)).toEqual({ forward: 3 });
    expect(store.read("vehicle.brake", 110)).toBe(1);

    store.setStale(true);
    expect(store.read("system.stale", 111)).toBe(true);
  });

  it("clears interpolation history when the game session changes", () => {
    const store = new TelemetryStore({ expectedSampleIntervalMs: 20 });
    store.ingest(snapshot(1, { sessionId: "a", rpm: 1_000 }), 0);
    store.ingest(snapshot(2, { sessionId: "a", rpm: 5_000 }), 10);
    expect(store.read("vehicle.rpm", 15)).toBe(2_000);

    store.ingest(snapshot(3, { sessionId: "b", rpm: 7_000 }), 15);
    expect(store.read("vehicle.rpm", 15)).toBe(7_000);
  });

  it("publishes a game change once and resets interpolation across sources", () => {
    const store = new TelemetryStore({ expectedSampleIntervalMs: 20 });
    store.ingest(snapshot(1, { gameId: "f1-25", sessionId: "same", rpm: 1_000 }), 0);
    store.ingest(snapshot(2, { gameId: "f1-25", sessionId: "same", rpm: 5_000 }), 10);
    expect(store.read("vehicle.rpm", 15)).toBe(2_000);

    const game = vi.fn();
    store.subscribe(["meta.gameId"], game);
    store.ingest(snapshot(3, { gameId: "ets2", sessionId: "same", rpm: 1_500 }), 15);

    expect(store.read("meta.gameId", 15)).toBe("ets2");
    expect(store.read("vehicle.rpm", 15)).toBe(1_500);
    expect(game).toHaveBeenCalledOnce();

    store.ingest(snapshot(4, { gameId: "ets2", sessionId: "same", rpm: 1_600 }), 20);
    expect(game).toHaveBeenCalledOnce();
  });

  it("notifies only subscribers whose field targets changed", () => {
    const store = new TelemetryStore({ expectedSampleIntervalMs: 20 });
    store.ingest(
      snapshot(1, { sessionId: "a", speedMps: 20, rpm: 1_000, gear: { forward: 2 } }),
      0,
    );
    const tachometer = vi.fn();
    const speed = vi.fn();
    const gear = vi.fn();
    store.subscribe(["vehicle.rpm"], tachometer);
    store.subscribe(["vehicle.speedMps"], speed);
    store.subscribe(["vehicle.gear"], gear);

    store.ingest(
      snapshot(2, { sessionId: "a", speedMps: 20, rpm: 2_000, gear: { forward: 2 } }),
      20,
    );

    expect(tachometer).toHaveBeenCalledOnce();
    expect(speed).not.toHaveBeenCalled();
    expect(gear).not.toHaveBeenCalled();
  });

  it("exposes the expanded F1 and SCS telemetry groups without losing native units", () => {
    const store = new TelemetryStore();
    store.ingest(
      {
        seq: 1,
        capturedAtUs: 1_000,
        data: {
          meta: {
            capturedAt: 1_000,
            gameId: "f1-25",
            schemaVersion: 1,
            sequence: 1,
            sessionId: "expanded",
          },
          vehicle: {
            fuelCapacityKg: 110,
            fuelKg: 42,
            fuelRemainingLaps: 9.2,
          },
          lap: { current: 12, deltaToBestMs: -84, position: 4 },
          session: { totalLaps: 57, trackId: "bahrain" },
          tyres: { frontLeft: { innerTemperatureC: 94, pressurePa: 145_000, wear: 0.12 } },
          conditions: { airTemperatureC: 28, trackTemperatureC: 37, weather: "clear" },
          damage: { engine: 0.03 },
          aero: { mode: "straight", overtakeAvailable: true, regulations2026: true },
          navigation: { distanceM: 286_400, speedLimitMps: 22.22, timeS: 15_120 },
          lights: { lowBeam: true },
          job: { active: true, cargo: "Electronics", destinationCity: "Prague" },
        },
      },
      1,
    );

    expect(store.read("vehicle.fuel", 1)).toMatchObject({ kg: 42, remainingLaps: 9.2 });
    expect(store.read("lap", 1)?.position).toBe(4);
    expect(store.read("session", 1)?.totalLaps).toBe(57);
    expect(store.read("tyres", 1)?.frontLeft?.pressurePa).toBe(145_000);
    expect(store.read("conditions", 1)?.weather).toBe("clear");
    expect(store.read("aero", 1)?.mode).toBe("straight");
    expect(store.read("navigation", 1)?.distanceM).toBe(286_400);
    expect(store.read("lights", 1)?.lowBeam).toBe(true);
    expect(store.read("job", 1)?.destinationCity).toBe("Prague");
    expect(store.read("system.stale", 1)).toBe(false);
  });
});
