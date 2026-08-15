import { defineWidgetManifest } from "@opensimdash/widget-sdk";

export const speedManifest = defineWidgetManifest({
  schemaVersion: 1,
  type: "core.speed",
  displayName: "Speed",
  description: "Vehicle speed with widget-owned unit conversion",
  fields: ["vehicle.speedMps"],
  minimumSize: { columns: 2, rows: 1 },
  defaultSize: { columns: 3, rows: 2 },
  defaultSettings: { unit: "km/h" as const },
});
