import { useEffect, useRef } from "preact/hooks";

import type { TelemetryRenderLoop } from "../../telemetry/render-loop";
import type { TelemetryValueMap } from "../../telemetry/store";
import { outputElements, setActive, setOutput } from "../dom-output";
import {
  formatDistance,
  formatDuration,
  formatFuel,
  fuelRatio,
  textOrDash,
} from "../telemetry-format";

export { routeManifest } from "./manifest";

type JobState = TelemetryValueMap["job"];
type LightsState = TelemetryValueMap["lights"];

export interface RouteWidgetProps {
  readonly loop: TelemetryRenderLoop;
}

export function RouteWidget({ loop }: RouteWidgetProps) {
  const root = useRef<HTMLElement>(null);

  useEffect(() => {
    const element = root.current;
    if (!element) {
      return;
    }
    const outputs = outputElements(element);
    return loop.bind(["navigation", "job", "lights", "vehicle.fuel"], (store, nowMs) => {
      const navigation = store.read("navigation", nowMs);
      const job = store.read("job", nowMs);
      const lights = store.read("lights", nowMs);
      const fuel = store.read("vehicle.fuel", nowMs);
      const ratio = fuelRatio(fuel);

      setOutput(outputs, "destination", job?.active === true ? textOrDash(job.destinationCity) : "FREE DRIVE");
      setOutput(outputs, "job-state", job?.special === true ? "SPECIAL" : job?.active === true ? "ON JOB" : "NO JOB");
      setOutput(outputs, "distance", formatDistance(navigation?.distanceM));
      setOutput(outputs, "eta", formatDuration(navigation?.timeS));
      setOutput(outputs, "limit", speedLimit(navigation?.speedLimitMps));
      setOutput(outputs, "fuel", ratio === undefined ? formatFuel(fuel) : `${Math.round(ratio * 100)}%`);
      setOutput(outputs, "route", routeLabel(job));
      setOutput(outputs, "cargo", cargoLabel(job));
      setOutput(outputs, "range", fuel?.rangeKm === undefined ? "" : `RANGE ${Math.round(fuel.rangeKm)} KM`);
      updateLights(outputs, lights);

      element.dataset.fuel = fuel?.warning === true || (ratio !== undefined && ratio < 0.15)
        ? "low"
        : ratio !== undefined && ratio < 0.4
          ? "medium"
          : ratio === undefined
            ? "unknown"
            : "high";
    });
  }, [loop]);

  return (
    <section ref={root} class="dashboard-widget route-widget" aria-label="Route and delivery telemetry">
      <header class="telemetry-panel-header route-header">
        <span class="widget-kicker">ROUTE</span>
        <output data-value="destination">FREE DRIVE</output>
        <output data-value="job-state">NO JOB</output>
      </header>
      <div class="route-metrics">
        <Metric label="DIST" value="distance" />
        <Metric label="ETA" value="eta" />
        <Metric label="LIMIT" value="limit" />
        <Metric label="FUEL" value="fuel" />
      </div>
      <div class="route-job">
        <output data-value="route">—</output>
        <output data-value="cargo">—</output>
        <output data-value="range" />
      </div>
      <div class="route-lights" aria-label="Exterior lights">
        <Light value="left" label="◀" />
        <Light value="low" label="LOW" />
        <Light value="high" label="HIGH" />
        <Light value="hazard" label="HAZ" />
        <Light value="beacon" label="BCN" />
        <Light value="right" label="▶" />
      </div>
    </section>
  );
}

function Metric({ label, value }: { readonly label: string; readonly value: string }) {
  return (
    <div class="telemetry-metric">
      <span>{label}</span>
      <output data-value={value}>—</output>
    </div>
  );
}

function Light({ value, label }: { readonly value: string; readonly label: string }) {
  return <output data-value={value}>{label}</output>;
}

function updateLights(outputs: ReturnType<typeof outputElements>, lights: LightsState): void {
  setActive(outputs, "left", lights?.leftIndicator === true || lights?.hazard === true);
  setActive(outputs, "low", lights?.lowBeam === true);
  setActive(outputs, "high", lights?.highBeam === true);
  setActive(outputs, "hazard", lights?.hazard === true);
  setActive(outputs, "beacon", lights?.beacon === true);
  setActive(outputs, "right", lights?.rightIndicator === true || lights?.hazard === true);
}

function speedLimit(metresPerSecond: number | null | undefined): string {
  return typeof metresPerSecond === "number" && Number.isFinite(metresPerSecond)
    ? `${Math.round(metresPerSecond * 3.6)}`
    : "—";
}

function routeLabel(job: JobState): string {
  if (job?.active !== true) {
    return "NO ACTIVE DELIVERY";
  }
  return `${textOrDash(job.sourceCity)} → ${textOrDash(job.destinationCity)}`;
}

function cargoLabel(job: JobState): string {
  if (job?.active !== true) {
    return "";
  }
  const mass = typeof job.cargoMassKg === "number" && Number.isFinite(job.cargoMassKg)
    ? `${(job.cargoMassKg / 1_000).toFixed(1)} T`
    : undefined;
  return [textOrDash(job.cargo), mass].filter((value) => value && value !== "—").join(" · ");
}
