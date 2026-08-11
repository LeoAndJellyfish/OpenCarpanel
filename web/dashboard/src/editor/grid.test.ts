import { describe, expect, it } from "vitest";

import { findAvailablePlacement, movePlacement, resizePlacement } from "./grid";

const grid = { columns: 12, rows: 8 };

describe("editor grid engine", () => {
  it("moves on integer cells and clamps to every canvas edge", () => {
    const placement = { x: 4, y: 2, width: 4, height: 3 };
    expect(movePlacement(placement, { columns: -20, rows: 20 }, grid, [])).toEqual({
      x: 0,
      y: 5,
      width: 4,
      height: 3,
    });
  });

  it("rejects a move or resize that would collide", () => {
    const placement = { x: 0, y: 0, width: 3, height: 2 };
    const occupied = [{ x: 3, y: 0, width: 3, height: 3 }];
    expect(movePlacement(placement, { columns: 3, rows: 0 }, grid, occupied)).toBe(placement);
    expect(
      resizePlacement(
        placement,
        { columns: 2, rows: 0 },
        grid,
        { columns: 2, rows: 1 },
        occupied,
      ),
    ).toBe(placement);
  });

  it("resizes within minimum size and canvas bounds", () => {
    const placement = { x: 8, y: 5, width: 3, height: 2 };
    expect(
      resizePlacement(
        placement,
        { columns: 20, rows: -20 },
        grid,
        { columns: 2, rows: 1 },
        [],
      ),
    ).toEqual({ x: 8, y: 5, width: 4, height: 1 });
  });

  it("finds the first deterministic non-overlapping slot", () => {
    expect(
      findAvailablePlacement(
        { columns: 3, rows: 2 },
        grid,
        [{ x: 0, y: 0, width: 5, height: 2 }],
      ),
    ).toEqual({ x: 5, y: 0, width: 3, height: 2 });
  });
});
