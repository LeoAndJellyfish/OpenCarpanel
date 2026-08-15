import type { BreakpointName, LayoutDocument } from "@opensimdash/widget-sdk";
import { useEffect, useRef } from "preact/hooks";

import type { ConnectionView } from "../connection/client";
import type { TelemetryRenderLoop } from "../telemetry/render-loop";
import type { GamePresentation } from "./game-profile";
import { LayoutGrid } from "./layout-grid";
import { dashboardThemeStyle, themePresetId } from "./theme";

export interface DashboardProps {
  readonly loop: TelemetryRenderLoop;
  readonly connection: ConnectionView;
  readonly layout: LayoutDocument;
  readonly breakpoint: BreakpointName;
  readonly presentation: GamePresentation;
}

export function Dashboard({
  loop,
  connection,
  layout,
  breakpoint,
  presentation,
}: DashboardProps) {
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
      data-game={presentation.id}
      data-game-family={presentation.family}
      style={dashboardThemeStyle(layout.theme)}
    >
      <div class="dashboard-frame" aria-hidden="true" />
      <LayoutGrid
        key={presentation.layoutId}
        layout={layout}
        breakpoint={breakpoint}
        loop={loop}
        connection={connection}
        statusMode={presentation.statusMode}
      />
      <footer class="drive-footer">
        <span>{presentation.label} / {presentation.detail}</span>
        <span>OPENSIMDASH — 01</span>
      </footer>
    </main>
  );
}
