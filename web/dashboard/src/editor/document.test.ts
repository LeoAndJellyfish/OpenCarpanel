import { describe, expect, it } from "vitest";

import { DEFAULT_LAYOUT } from "@opensimdash/widget-sdk";

import { gearManifest } from "../widgets/gear";
import {
  addWidget,
  duplicateWidget,
  removeWidget,
  removeWidgetsByType,
} from "./document";

describe("layout document editing", () => {
  it("adds and duplicates registered widgets at deterministic free positions", () => {
    const editable = {
      ...DEFAULT_LAYOUT,
      widgets: DEFAULT_LAYOUT.widgets.filter(
        (widget) => widget.componentType !== "core.race" && widget.componentType !== "core.tyres",
      ),
    };
    const added = addWidget(editable, gearManifest);
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

  it("removes every instance of a disabled component type", () => {
    const withDuplicate = {
      ...DEFAULT_LAYOUT,
      widgets: [
        ...DEFAULT_LAYOUT.widgets,
        {
          ...DEFAULT_LAYOUT.widgets.find((widget) => widget.instanceId === "speed")!,
          instanceId: "speed-2",
        },
      ],
    };
    const removed = removeWidgetsByType(withDuplicate, "core.speed");

    expect(removed.widgets.some((widget) => widget.componentType === "core.speed")).toBe(false);
    expect(removed.widgets).toHaveLength(DEFAULT_LAYOUT.widgets.length - 1);
  });
});
