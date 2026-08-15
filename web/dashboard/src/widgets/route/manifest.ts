import { defineWidgetManifest } from "@opensimdash/widget-sdk";

export const routeManifest = defineWidgetManifest({
  schemaVersion: 1,
  type: "core.route",
  displayName: "Route & job",
  description: "Navigation, delivery, fuel and exterior-light state",
  fields: ["navigation", "job", "lights", "vehicle.fuel"],
  minimumSize: { columns: 4, rows: 3 },
  defaultSize: { columns: 5, rows: 5 },
  defaultSettings: {},
});
