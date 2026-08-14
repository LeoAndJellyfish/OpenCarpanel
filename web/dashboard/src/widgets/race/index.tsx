import { useEffect, useRef } from "preact/hooks";

import type { TelemetryRenderLoop } from "../../telemetry/render-loop";
import type { TelemetryValueMap } from "../../telemetry/store";
import { outputElements, setActive, setOutput } from "../dom-output";
import {
  formatDelta,
  formatFuel,
  formatLapTime,
  formatSessionTime,
  textOrDash,
  weatherLabel,
} from "../telemetry-format";

export { raceManifest } from "./manifest";

export interface RaceWidgetProps {
  readonly loop: TelemetryRenderLoop;
}

export function RaceWidget({ loop }: RaceWidgetProps) {
  const root = useRef<HTMLElement>(null);

  useEffect(() => {
    const element = root.current;
    if (!element) {
      return;
    }
    const outputs = outputElements(element);
    return loop.bind(
      ["lap", "session", "conditions", "aero", "vehicle.fuel"],
      (store, nowMs) => {
        const lap = store.read("lap", nowMs);
        const session = store.read("session", nowMs);
        const conditions = store.read("conditions", nowMs);
        const aero = store.read("aero", nowMs);
        const fuel = store.read("vehicle.fuel", nowMs);
        const position = finiteInteger(lap?.position);
        const currentLap = finiteInteger(lap?.current);
        const totalLaps = finiteInteger(session?.totalLaps);
        const penaltySeconds = finiteNumber(lap?.penaltiesSeconds);
        const invalid = lap?.invalid === true;

        setOutput(outputs, "track", textOrDash(session?.trackId));
        setOutput(outputs, "session", sessionLabel(session?.sessionType));
        setOutput(outputs, "position", position === undefined ? "—" : `P${position.toString().padStart(2, "0")}`);
        setOutput(outputs, "lap", `${currentLap ?? "—"} / ${totalLaps ?? "—"}`);
        setOutput(outputs, "delta", formatDelta(lap?.deltaToBestMs));
        setOutput(outputs, "fuel", raceFuelLabel(fuel));
        setOutput(outputs, "current-time", `NOW ${formatLapTime(lap?.currentTimeMs)}`);
        setOutput(outputs, "last-time", `LAST ${formatLapTime(lap?.lastTimeMs)}`);
        setOutput(outputs, "remaining", `LEFT ${formatSessionTime(session?.remainingTimeMs)}`);
        setOutput(outputs, "conditions", conditionsLabel(conditions));
        setOutput(outputs, "control", raceControlLabel(session?.raceFlag, session?.safetyCarStatus));
        setOutput(outputs, "aero", aeroLabel(aero));
        setOutput(outputs, "penalty", penaltySeconds && penaltySeconds > 0 ? `+${penaltySeconds} SEC` : invalid ? "LAP INVALID" : "CLEAN LAP");
        setActive(outputs, "control", session?.raceFlag === "green");
        setActive(outputs, "aero", aero?.overtakeActive === true || aero?.mode === "straight");
        element.dataset.warning = invalid || fuel?.warning === true || (penaltySeconds ?? 0) > 0 ? "true" : "false";
      },
    );
  }, [loop]);

  return (
    <section ref={root} class="dashboard-widget race-widget" aria-label="Race telemetry">
      <header class="telemetry-panel-header">
        <span class="widget-kicker">RACE</span>
        <output data-value="track">—</output>
        <output data-value="session">—</output>
      </header>
      <div class="race-metrics">
        <Metric label="POS" value="position" />
        <Metric label="LAP" value="lap" />
        <Metric label="DELTA" value="delta" />
        <Metric label="FUEL" value="fuel" />
      </div>
      <div class="telemetry-panel-meta race-timing">
        <output data-value="current-time">NOW —</output>
        <output data-value="last-time">LAST —</output>
        <output data-value="remaining">LEFT —</output>
      </div>
      <div class="telemetry-panel-meta race-context">
        <output data-value="conditions">—</output>
        <output data-value="control">—</output>
        <output data-value="aero">—</output>
        <output data-value="penalty">CLEAN LAP</output>
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

function conditionsLabel(conditions: TelemetryValueMap["conditions"]): string {
  const air = finiteNumber(conditions?.airTemperatureC);
  const track = finiteNumber(conditions?.trackTemperatureC);
  const temperatures = [air === undefined ? undefined : `AIR ${Math.round(air)}°`, track === undefined ? undefined : `TRACK ${Math.round(track)}°`]
    .filter((value): value is string => value !== undefined)
    .join(" · ");
  return [weatherLabel(conditions?.weather), temperatures].filter((value) => value && value !== "—").join(" · ") || "—";
}

function raceControlLabel(flag: string | null | undefined, safetyCar: string | null | undefined): string {
  if (safetyCar && safetyCar !== "none") {
    return safetyCar === "virtual" ? "VSC" : safetyCar.replaceAll("_", " ").toUpperCase();
  }
  return flag && flag !== "none" ? `${flag.toUpperCase()} FLAG` : "GREEN";
}

function aeroLabel(aero: {
  readonly regulations2026?: boolean | null;
  readonly mode?: string | null;
  readonly available?: boolean | null;
  readonly overtakeActive?: boolean | null;
  readonly overtakeAvailable?: boolean | null;
} | undefined): string {
  if (aero?.regulations2026 !== true) {
    return "";
  }
  if (aero.overtakeActive === true) {
    return "OVERTAKE ACTIVE";
  }
  if (aero.overtakeAvailable === true) {
    return "OVERTAKE READY";
  }
  if (aero.mode) {
    return `AERO ${aero.mode.toUpperCase()}`;
  }
  return aero.available === true ? "AERO READY" : "AERO CORNER";
}

function sessionLabel(value: string | null | undefined): string {
  return value ? value.replaceAll("_", " ").toUpperCase() : "SESSION";
}

function raceFuelLabel(fuel: TelemetryValueMap["vehicle.fuel"]): string {
  const remainingLaps = finiteNumber(fuel?.remainingLaps);
  return remainingLaps === undefined ? formatFuel(fuel) : `${remainingLaps.toFixed(1)} LP`;
}

function finiteNumber(value: unknown): number | undefined {
  return typeof value === "number" && Number.isFinite(value) ? value : undefined;
}

function finiteInteger(value: unknown): number | undefined {
  return Number.isInteger(value) && Number(value) >= 0 ? Number(value) : undefined;
}
