import { defineWidgetManifest } from "@opensimdash/widget-sdk";

export const gearManifest = defineWidgetManifest({
  schemaVersion: 1,
  type: "core.gear",
  displayName: "Gear",
  description: "Large immediate current-gear readout",
  fields: ["vehicle.gear"],
  minimumSize: { columns: 2, rows: 2 },
  defaultSize: { columns: 4, rows: 4 },
  defaultSettings: {},
});
