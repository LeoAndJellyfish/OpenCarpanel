import { useEffect, useRef } from "preact/hooks";

import type { ConnectionView } from "../connection/client";
import type { TelemetryRenderLoop } from "../telemetry/render-loop";
import type { StatusMode } from "./game-profile";

export interface StatusRailProps {
  readonly loop: TelemetryRenderLoop;
  readonly connection: ConnectionView;
  readonly mode: StatusMode;
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

export function StatusRail({ loop, connection, mode }: StatusRailProps) {
  const telemetry = useRef<HTMLOutputElement>(null);
  const source = useRef<HTMLOutputElement>(null);

  useEffect(
    () =>
      loop.bind(
        mode === "drs" ? ["system.stale", "vehicle.drs"] : ["system.stale"],
        (store, nowMs) => {
        const stale = store.read("system.stale", nowMs);
        if (telemetry.current) {
          telemetry.current.textContent = stale ? "DATA STALE" : "TELEMETRY LIVE";
          telemetry.current.dataset.active = stale ? "false" : "true";
        }
        if (source.current) {
          if (mode === "drs") {
            const drsState = store.read("vehicle.drs", nowMs);
            source.current.textContent =
              drsState === "active"
                ? "DRS ACTIVE"
                : drsState === "available"
                  ? "DRS READY"
                  : "DRS —";
            source.current.dataset.active = drsState === "active" ? "true" : "false";
          } else if (mode === "scs") {
            source.current.textContent = stale ? "SCS WAITING" : "SCS BRIDGE";
            source.current.dataset.active = stale ? "false" : "true";
          } else {
            source.current.textContent = stale ? "WAITING" : "GAME LINK";
            source.current.dataset.active = stale ? "false" : "true";
          }
        }
      },
      ),
    [loop, mode],
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
        <span>{mode === "drs" ? "AERO" : "SOURCE"}</span>
        <output ref={source}>
          {mode === "drs" ? "DRS —" : mode === "scs" ? "SCS WAITING" : "WAITING"}
        </output>
      </div>
    </aside>
  );
}
