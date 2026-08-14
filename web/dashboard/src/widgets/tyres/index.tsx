import { useEffect, useRef } from "preact/hooks";

import type { TelemetryRenderLoop } from "../../telemetry/render-loop";
import type { TelemetryValueMap } from "../../telemetry/store";
import { outputElements, setOutput } from "../dom-output";
import { formatPercent, formatPressure, formatTemperature } from "../telemetry-format";

export { tyresManifest } from "./manifest";

type TyreState = NonNullable<TelemetryValueMap["tyres"]>;
type TyreCorner = NonNullable<TyreState["frontLeft"]>;
type DamageState = NonNullable<TelemetryValueMap["damage"]>;

const COMPOUND_LABELS: Readonly<Record<number, string>> = {
  7: "INTER",
  8: "WET",
  16: "SOFT",
  17: "MEDIUM",
  18: "HARD",
};

export interface TyresWidgetProps {
  readonly loop: TelemetryRenderLoop;
}

export function TyresWidget({ loop }: TyresWidgetProps) {
  const root = useRef<HTMLElement>(null);

  useEffect(() => {
    const element = root.current;
    if (!element) {
      return;
    }
    const outputs = outputElements(element);
    return loop.bind(["tyres", "damage"], (store, nowMs) => {
      const tyres = store.read("tyres", nowMs);
      const damage = store.read("damage", nowMs);
      setOutput(outputs, "compound", compoundLabel(tyres?.visualCompound ?? tyres?.actualCompound));
      setOutput(outputs, "age", Number.isInteger(tyres?.ageLaps) ? `${tyres?.ageLaps} LAPS` : "—");
      updateCorner(outputs, "fl", tyres?.frontLeft ?? undefined);
      updateCorner(outputs, "fr", tyres?.frontRight ?? undefined);
      updateCorner(outputs, "rl", tyres?.rearLeft ?? undefined);
      updateCorner(outputs, "rr", tyres?.rearRight ?? undefined);

      const health = damageHealth(damage, tyres);
      setOutput(outputs, "damage", health.label);
      element.dataset.warning = health.warning ? "true" : "false";
    });
  }, [loop]);

  return (
    <section ref={root} class="dashboard-widget tyres-widget" aria-label="Tyre and vehicle health">
      <header class="telemetry-panel-header">
        <span class="widget-kicker">TYRES</span>
        <output data-value="compound">—</output>
        <output data-value="age">—</output>
      </header>
      <div class="tyre-grid">
        <TyreCornerView label="FL" keyName="fl" />
        <TyreCornerView label="FR" keyName="fr" />
        <TyreCornerView label="RL" keyName="rl" />
        <TyreCornerView label="RR" keyName="rr" />
      </div>
      <div class="tyre-health">
        <span>CAR</span>
        <output data-value="damage">—</output>
      </div>
    </section>
  );
}

function TyreCornerView({ label, keyName }: { readonly label: string; readonly keyName: string }) {
  return (
    <article class="tyre-corner">
      <span>{label}</span>
      <output class="tyre-temperature" data-value={`${keyName}-temperature`}>—</output>
      <output class="tyre-wear" data-value={`${keyName}-wear`}>—</output>
      <output class="tyre-pressure" data-value={`${keyName}-pressure`}>—</output>
    </article>
  );
}

function updateCorner(
  outputs: ReturnType<typeof outputElements>,
  key: string,
  corner: TyreCorner | undefined,
): void {
  setOutput(outputs, `${key}-temperature`, formatTemperature(corner?.innerTemperatureC ?? corner?.surfaceTemperatureC));
  setOutput(outputs, `${key}-wear`, formatPercent(corner?.wear));
  const pressure = formatPressure(corner?.pressurePa);
  setOutput(outputs, `${key}-pressure`, pressure === "—" ? pressure : `${pressure} BAR`);
}

function compoundLabel(value: number | null | undefined): string {
  return typeof value === "number" && Number.isFinite(value)
    ? (COMPOUND_LABELS[value] ?? `C${value}`)
    : "—";
}

function damageHealth(
  damage: DamageState | undefined,
  tyres: TyreState | undefined,
): { readonly label: string; readonly warning: boolean } {
  if (damage?.engineBlown === true || damage?.engineSeized === true) {
    return { label: "ENGINE", warning: true };
  }
  if (damage?.drsFault === true || damage?.ersFault === true) {
    return { label: "SYSTEM FAULT", warning: true };
  }
  const values = [
    damage?.diffuser,
    damage?.engine,
    damage?.floor,
    damage?.frontLeftWing,
    damage?.frontRightWing,
    damage?.gearbox,
    damage?.rearWing,
    damage?.sidepod,
    tyres?.frontLeft?.damage,
    tyres?.frontRight?.damage,
    tyres?.rearLeft?.damage,
    tyres?.rearRight?.damage,
  ].filter((value): value is number => typeof value === "number" && Number.isFinite(value));
  if (values.length === 0) {
    return { label: "—", warning: false };
  }
  const maximum = Math.max(...values);
  return { label: formatPercent(maximum), warning: maximum >= 0.25 };
}
