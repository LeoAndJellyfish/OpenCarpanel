import { describe, expect, it } from "vitest";

import { formatSpeed } from "./format";

describe("formatSpeed", () => {
  it("converts metres per second to stable kilometre-per-hour digits", () => {
    expect(formatSpeed(0)).toBe("000");
    expect(formatSpeed(27.777_778)).toBe("100");
    expect(formatSpeed(undefined)).toBe("—");
  });
});
