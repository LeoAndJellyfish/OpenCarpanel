import type { BreakpointName, LayoutDocument } from "@opencarpanel/widget-sdk";
import { useEffect, useRef } from "preact/hooks";

import type { ConnectionView } from "../connection/client";
import type { TelemetryRenderLoop } from "../telemetry/render-loop";
import { LayoutGrid } from "./layout-grid";
import { themePresetId } from "./theme";

export interface DashboardProps {
  readonly loop: TelemetryRenderLoop;
  readonly connection: ConnectionView;
  readonly layout: LayoutDocument;
  readonly breakpoint: BreakpointName;
}

export function Dashboard({ loop, connection, layout, breakpoint }: DashboardProps) {
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
    <main
      ref={root}
      class="drive-dashboard"
      data-stale="true"
      data-theme={themePresetId(layout.theme)}
    >
      <div class="dashboard-frame" aria-hidden="true" />
      <LayoutGrid
        layout={layout}
        breakpoint={breakpoint}
        loop={loop}
        connection={connection}
      />
      <footer class="drive-footer">
        <span>F1 24 / LOCAL TELEMETRY</span>
        <span>OPEN CARPANEL — 01</span>
      </footer>
    </main>
  );
}
