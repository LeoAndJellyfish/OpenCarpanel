import { useEffect, useRef } from "preact/hooks";

import type { ConnectionView } from "../connection/client";
import type { TelemetryRenderLoop } from "../telemetry/render-loop";

export interface StatusRailProps {
  readonly loop: TelemetryRenderLoop;
  readonly connection: ConnectionView;
}

function connectionLabel(phase: ConnectionView["phase"]): string {
  switch (phase) {
    case "connected":
      return "LINK ONLINE";
    case "connecting":
      return "LINKING";
    case "disconnected":
      return "RECONNECTING";
    case "pairing_required":
      return "PAIRING REQUIRED";
    case "error":
      return "LINK ERROR";
  }
}

export function StatusRail({ loop, connection }: StatusRailProps) {
  const telemetry = useRef<HTMLOutputElement>(null);
  const drs = useRef<HTMLOutputElement>(null);

  useEffect(
    () =>
      loop.bind(["system.stale", "vehicle.drs"], (store, nowMs) => {
        const stale = store.read("system.stale", nowMs);
        const drsState = store.read("vehicle.drs", nowMs);
        if (telemetry.current) {
          telemetry.current.textContent = stale ? "DATA STALE" : "TELEMETRY LIVE";
          telemetry.current.dataset.active = stale ? "false" : "true";
        }
        if (drs.current) {
          drs.current.textContent =
            drsState === "active"
              ? "DRS ACTIVE"
              : drsState === "available"
                ? "DRS READY"
                : "DRS —";
          drs.current.dataset.active = drsState === "active" ? "true" : "false";
        }
      }),
    [loop],
  );

  return (
    <aside class="dashboard-widget status-rail" aria-label="Telemetry status">
      <div class="status-line">
        <span>HOST</span>
        <strong data-phase={connection.phase}>{connectionLabel(connection.phase)}</strong>
      </div>
      <div class="status-line">
        <span>SIGNAL</span>
        <output ref={telemetry}>DATA STALE</output>
      </div>
      <div class="status-line">
        <span>AERO</span>
        <output ref={drs}>DRS —</output>
      </div>
    </aside>
  );
}
