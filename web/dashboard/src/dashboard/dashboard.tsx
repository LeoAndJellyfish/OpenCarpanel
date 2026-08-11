import { useEffect, useRef } from "preact/hooks";

import type { ConnectionView } from "../connection/client";
import type { TelemetryRenderLoop } from "../telemetry/render-loop";
import { GearWidget } from "../widgets/gear";
import { SpeedWidget } from "../widgets/speed";
import { TachometerWidget } from "../widgets/tachometer";
import { StatusRail } from "./status-rail";

export interface DashboardProps {
  readonly loop: TelemetryRenderLoop;
  readonly connection: ConnectionView;
}

export function Dashboard({ loop, connection }: DashboardProps) {
  const root = useRef<HTMLElement>(null);

  useEffect(
    () =>
      loop.bind(["system.stale"], (store, nowMs) => {
        if (root.current) {
          root.current.dataset.stale = store.read("system.stale", nowMs) ? "true" : "false";
        }
      }),
    [loop],
  );

  return (
    <main ref={root} class="drive-dashboard" data-stale="true">
      <div class="dashboard-frame" aria-hidden="true" />
      <TachometerWidget loop={loop} />
      <div class="primary-grid">
        <SpeedWidget loop={loop} />
        <GearWidget loop={loop} />
        <StatusRail loop={loop} connection={connection} />
      </div>
      <footer class="drive-footer">
        <span>F1 24 / LOCAL TELEMETRY</span>
        <span>OPEN CARPANEL — 01</span>
      </footer>
    </main>
  );
}
