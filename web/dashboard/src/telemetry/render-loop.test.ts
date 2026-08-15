import type { SnapshotMessage } from "@opensimdash/widget-sdk";
import { describe, expect, it, vi } from "vitest";

import { TelemetryRenderLoop, type FrameDriver } from "./render-loop";
import { TelemetryStore } from "./store";

class FakeFrameDriver implements FrameDriver {
  callback: FrameRequestCallback | undefined;
  requestCount = 0;

  request(callback: FrameRequestCallback): number {
    this.callback = callback;
    this.requestCount += 1;
    return this.requestCount;
  }

  cancel(): void {
    this.callback = undefined;
  }

  flush(nowMs: number): void {
    const callback = this.callback;
    this.callback = undefined;
    callback?.(nowMs);
  }
}

function snapshot(sequence: number, rpm: number): SnapshotMessage {
  return {
    seq: sequence,
    capturedAtUs: sequence * 1_000,
    data: {
      meta: { schemaVersion: 1, sequence, sessionId: "test" },
      vehicle: { speedMps: 20, rpm, rpmMax: 10_000, gear: { forward: 2 } },
    },
  };
}

describe("TelemetryRenderLoop", () => {
  it("uses one frame request and invokes only bindings affected by an RPM update", () => {
    const store = new TelemetryStore({ expectedSampleIntervalMs: 20 });
    const driver = new FakeFrameDriver();
    const loop = new TelemetryRenderLoop(store, driver);
    const tachometer = vi.fn();
    const speed = vi.fn();
    const gear = vi.fn();
    loop.bind(["vehicle.rpm"], tachometer);
    loop.bind(["vehicle.speedMps"], speed);
    loop.bind(["vehicle.gear"], gear);
    driver.flush(0);
    tachometer.mockClear();
    speed.mockClear();
    gear.mockClear();

    store.ingest(snapshot(1, 1_000), 0);
    driver.flush(0);
    tachometer.mockClear();
    speed.mockClear();
    gear.mockClear();
    store.ingest(snapshot(2, 5_000), 20);
    driver.flush(20);

    expect(tachometer).toHaveBeenCalledOnce();
    expect(speed).not.toHaveBeenCalled();
    expect(gear).not.toHaveBeenCalled();
    loop.destroy();
  });
});
