import { defineWidgetManifest } from "@opensimdash/widget-sdk";

export const tyresManifest = defineWidgetManifest({
  schemaVersion: 1,
  type: "core.tyres",
  displayName: "Tyres & damage",
  description: "Four-corner temperature, pressure, wear and vehicle health",
  fields: ["tyres", "damage"],
  minimumSize: { columns: 4, rows: 2 },
  defaultSize: { columns: 5, rows: 3 },
  defaultSettings: {},
});
