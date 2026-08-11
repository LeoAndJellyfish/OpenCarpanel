import type { Gear, SnapshotMessage } from "@opencarpanel/widget-sdk";
import { describe, expect, it, vi } from "vitest";

import { TelemetryStore } from "./store";

interface SnapshotValues {
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
});
