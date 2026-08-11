import { describe, expect, it } from "vitest";

import { formatGear } from "./format";

describe("formatGear", () => {
  it("formats forward, neutral, reverse, and missing values without invention", () => {
    expect(formatGear({ forward: 7 })).toBe("7");
    expect(formatGear("neutral")).toBe("N");
    expect(formatGear("reverse")).toBe("R");
    expect(formatGear(undefined)).toBe("–");
  });
});
