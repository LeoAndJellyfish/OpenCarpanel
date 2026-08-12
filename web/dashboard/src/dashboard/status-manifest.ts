import { defineWidgetManifest } from "@opencarpanel/widget-sdk";

export const statusManifest = defineWidgetManifest({
  schemaVersion: 1,
  type: "core.status",
  displayName: "Telemetry status",
  description: "Host link, signal freshness and game-specific source state",
  fields: ["vehicle.drs"],
  minimumSize: { columns: 3, rows: 2 },
  defaultSize: { columns: 4, rows: 4 },
  defaultSettings: {},
});
