import { describe, expect, it } from "vitest";

import {
  BREAKPOINT_GRIDS,
  BREAKPOINT_NAMES,
  DEFAULT_LAYOUT,
  cloneLayout,
  parseLayoutDocument,
} from "./layout";

describe("layout contract", () => {
  it("keeps every default widget inside every breakpoint grid without collisions", () => {
    for (const breakpoint of BREAKPOINT_NAMES) {
      const grid = BREAKPOINT_GRIDS[breakpoint];
      const placements = DEFAULT_LAYOUT.widgets.map((widget) => widget.placements[breakpoint]);
      for (const placement of placements) {
        expect(placement).toBeDefined();
        expect(placement!.x + placement!.width).toBeLessThanOrEqual(grid.columns);
        expect(placement!.y + placement!.height).toBeLessThanOrEqual(grid.rows);
      }
      for (let leftIndex = 0; leftIndex < placements.length; leftIndex += 1) {
        for (let rightIndex = leftIndex + 1; rightIndex < placements.length; rightIndex += 1) {
          const left = placements[leftIndex]!;
          const right = placements[rightIndex]!;
          const overlaps =
            left.x < right.x + right.width &&
            left.x + left.width > right.x &&
            left.y < right.y + right.height &&
            left.y + left.height > right.y;
          expect(overlaps).toBe(false);
        }
      }
    }
  });

  it("parses and clones the generated-schema-shaped document", () => {
    const cloned = cloneLayout(DEFAULT_LAYOUT);
    expect(parseLayoutDocument(cloned)).toEqual(DEFAULT_LAYOUT);
    expect(cloned).not.toBe(DEFAULT_LAYOUT);
    expect(cloned.widgets).not.toBe(DEFAULT_LAYOUT.widgets);
  });

  it("rejects unsafe versions, colors and duplicate widget ids", () => {
    expect(() =>
      parseLayoutDocument({ ...DEFAULT_LAYOUT, schemaVersion: 2 }),
    ).toThrowError("unsupported layout schema version");
    expect(() =>
      parseLayoutDocument({
        ...DEFAULT_LAYOUT,
        theme: { ...DEFAULT_LAYOUT.theme, accent: "url(javascript:bad)" },
      }),
    ).toThrowError("hexadecimal color");
    expect(() =>
      parseLayoutDocument({
        ...DEFAULT_LAYOUT,
        widgets: [DEFAULT_LAYOUT.widgets[0], DEFAULT_LAYOUT.widgets[0]],
      }),
    ).toThrowError("duplicate widget instance");
  });
});
