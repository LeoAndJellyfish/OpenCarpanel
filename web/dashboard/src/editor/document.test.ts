import { describe, expect, it } from "vitest";

import { DEFAULT_LAYOUT } from "@opencarpanel/widget-sdk";

import { gearManifest } from "../widgets/gear";
import { addWidget, duplicateWidget, removeWidget } from "./document";

describe("layout document editing", () => {
  it("adds and duplicates registered widgets at deterministic free positions", () => {
    const added = addWidget(DEFAULT_LAYOUT, gearManifest);
    expect(added).toBeDefined();
    expect(added!.widgets.at(-1)?.instanceId).toBe("gear-2");
    const duplicated = duplicateWidget(added!, "gear", gearManifest);
    expect(duplicated).toBeDefined();
    expect(duplicated!.widgets.at(-1)?.instanceId).toBe("gear-3");
  });

  it("removes only the selected stable instance", () => {
    const removed = removeWidget(DEFAULT_LAYOUT, "speed");
    expect(removed.widgets.some((widget) => widget.instanceId === "speed")).toBe(false);
    expect(removed.widgets).toHaveLength(DEFAULT_LAYOUT.widgets.length - 1);
  });
});
