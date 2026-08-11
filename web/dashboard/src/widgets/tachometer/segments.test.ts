import { describe, expect, it } from "vitest";

import { REV_SEGMENT_COUNT, activationRank, activeSegmentCount } from "./segments";

describe("tachometer segments", () => {
  it("clamps rev-light progression and fills from both outside edges", () => {
    expect(activeSegmentCount(undefined)).toBe(0);
    expect(activeSegmentCount(-1)).toBe(0);
    expect(activeSegmentCount(0.5)).toBe(REV_SEGMENT_COUNT / 2);
    expect(activeSegmentCount(2)).toBe(REV_SEGMENT_COUNT);
    expect(activationRank(0)).toBe(0);
    expect(activationRank(REV_SEGMENT_COUNT - 1)).toBe(1);
  });
});
