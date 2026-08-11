import { describe, expect, it } from "vitest";

import { interpolateLinear } from "./interpolate";

describe("bounded telemetry interpolation", () => {
  it("interpolates for one expected sample interval and then clamps", () => {
    const sample = {
      from: 1_000,
      to: 5_000,
      startedAtMs: 100,
      durationMs: 20,
    };

    expect(interpolateLinear(sample, 90)).toBe(1_000);
    expect(interpolateLinear(sample, 110)).toBe(3_000);
    expect(interpolateLinear(sample, 120)).toBe(5_000);
    expect(interpolateLinear(sample, 500)).toBe(5_000);
  });

  it("returns the target immediately when interpolation is disabled", () => {
    expect(
      interpolateLinear({ from: 10, to: 20, startedAtMs: 5, durationMs: 0 }, 5),
    ).toBe(20);
  });
});
