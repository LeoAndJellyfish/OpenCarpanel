import { defineWidgetManifest } from "@opensimdash/widget-sdk";

export const tachometerManifest = defineWidgetManifest({
  schemaVersion: 1,
  type: "core.tachometer",
  displayName: "Shift horizon",
  description: "F1 rev-light progression with an RPM fallback",
  fields: ["vehicle.rpm", "vehicle.rpmMax", "vehicle.revLights"],
  minimumSize: { columns: 4, rows: 1 },
  defaultSize: { columns: 12, rows: 2 },
  defaultSettings: { fallbackRpmMax: 12_000 },
});
