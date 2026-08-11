import { describe, expect, it } from "vitest";

import { defineWidgetManifest } from "./manifest";
import { WidgetRegistry } from "./registry";

function manifest(type: `core.${string}`) {
  return defineWidgetManifest({
    schemaVersion: 1,
    type,
    displayName: type,
    description: "test widget",
    fields: ["vehicle.rpm"],
    minimumSize: { columns: 1, rows: 1 },
    defaultSize: { columns: 2, rows: 1 },
    defaultSettings: {},
  });
}

describe("WidgetRegistry", () => {
  it("keeps built-ins in stable type order and rejects duplicates", () => {
    const registry = new WidgetRegistry<string>();
    registry.register({ manifest: manifest("core.speed"), implementation: "speed" });
    registry.register({ manifest: manifest("core.gear"), implementation: "gear" });

    expect(registry.list().map((entry) => entry.manifest.type)).toEqual([
      "core.gear",
      "core.speed",
    ]);
    expect(() =>
      registry.register({ manifest: manifest("core.gear"), implementation: "duplicate" }),
    ).toThrowError("widget type core.gear is already registered");
  });
});
