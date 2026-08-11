import { describe, expect, it } from "vitest";

import { commitHistory, createHistory, redoHistory, undoHistory } from "./history";

describe("editor history", () => {
  it("restores exact immutable JSON documents through undo and redo", () => {
    const initial = { revision: 1, widgets: [{ id: "gear", x: 4 }] };
    const moved = { revision: 1, widgets: [{ id: "gear", x: 5 }] };
    const resized = { revision: 1, widgets: [{ id: "gear", x: 5, width: 4 }] };
    const history = commitHistory(commitHistory(createHistory(initial), moved), resized);
    const undone = undoHistory(history);
    expect(undone.present).toBe(moved);
    expect(undoHistory(undone).present).toBe(initial);
    expect(redoHistory(undone).present).toBe(resized);
  });

  it("clears redo state on a new edit and bounds history", () => {
    let history = createHistory({ value: 0 });
    for (let value = 1; value <= 6; value += 1) {
      history = commitHistory(history, { value }, 3);
    }
    expect(history.past).toHaveLength(3);
    const undone = undoHistory(history);
    expect(commitHistory(undone, { value: 99 }).future).toHaveLength(0);
  });
});
