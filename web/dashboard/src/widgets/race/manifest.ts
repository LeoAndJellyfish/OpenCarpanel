import { defineWidgetManifest } from "@opensimdash/widget-sdk";

export const raceManifest = defineWidgetManifest({
  schemaVersion: 1,
  type: "core.race",
  displayName: "Race",
  description: "Position, lap timing, fuel, conditions and race control",
  fields: [
    "lap.current",
    "lap.position",
    "lap.currentTimeMs",
    "lap.lastTimeMs",
    "lap.deltaToBestMs",
    "lap.invalid",
    "session.trackId",
    "session.remainingTimeMs",
    "session.totalLaps",
    "conditions",
    "aero",
    "vehicle.fuel",
  ],
  minimumSize: { columns: 4, rows: 2 },
  defaultSize: { columns: 7, rows: 3 },
  defaultSettings: {},
});
