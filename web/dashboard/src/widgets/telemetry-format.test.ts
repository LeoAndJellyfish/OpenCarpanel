import { describe, expect, it } from "vitest";

import {
  formatDelta,
  formatDistance,
  formatDuration,
  formatFuel,
  formatLapTime,
  formatPercent,
  fuelRatio,
  weatherLabel,
} from "./telemetry-format";

describe("telemetry formatting", () => {
  it("formats racing timing without losing millisecond precision", () => {
    expect(formatLapTime(91_422)).toBe("1:31.422");
    expect(formatDelta(184)).toBe("+0.184");
    expect(formatDelta(-23)).toBe("−0.023");
    expect(formatLapTime(undefined)).toBe("—");
  });

  it("uses compact route units", () => {
    expect(formatDistance(450)).toBe("450 M");
    expect(formatDistance(12_340)).toBe("12.3 KM");
    expect(formatDuration(15_120)).toBe("4 H 12 M");
  });

  it("chooses the most useful available fuel representation", () => {
    expect(formatFuel({
      capacityKg: 110,
      capacityLiters: undefined,
      kg: 42,
      liters: undefined,
      rangeKm: undefined,
      remainingLaps: 8.35,
      warning: false,
    })).toBe("8.3 LAPS");
    expect(fuelRatio({
      capacityKg: undefined,
      capacityLiters: 800,
      kg: undefined,
      liters: 320,
      rangeKm: 612,
      remainingLaps: undefined,
      warning: false,
    })).toBe(0.4);
  });

  it("normalizes percentages and weather labels", () => {
    expect(formatPercent(0.128)).toBe("13%");
    expect(formatPercent(2)).toBe("100%");
    expect(weatherLabel("heavy_rain")).toBe("HEAVY RAIN");
  });
});
