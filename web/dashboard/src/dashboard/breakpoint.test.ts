import { describe, expect, it } from "vitest";

import { selectBreakpoint } from "./breakpoint";

describe("dashboard breakpoints", () => {
  it("selects phone orientation, tablet and desktop deterministically", () => {
    expect(selectBreakpoint(390, 844)).toBe("phonePortrait");
    expect(selectBreakpoint(844, 390)).toBe("phoneLandscape");
    expect(selectBreakpoint(1_024, 768)).toBe("tablet");
    expect(selectBreakpoint(1_440, 900)).toBe("desktop");
  });
});
