import { useEffect, useRef } from "preact/hooks";

import type { TelemetryRenderLoop } from "../../telemetry/render-loop";
import { formatSpeed } from "./format";

export { speedManifest } from "./manifest";

export interface SpeedWidgetProps {
  readonly loop: TelemetryRenderLoop;
}

export function SpeedWidget({ loop }: SpeedWidgetProps) {
  const value = useRef<HTMLOutputElement>(null);

  useEffect(
    () =>
      loop.bind(["vehicle.speedMps"], (store, nowMs) => {
        if (value.current) {
          value.current.textContent = formatSpeed(store.read("vehicle.speedMps", nowMs));
        }
      }),
    [loop],
  );

  return (
    <section class="dashboard-widget speed-widget" aria-label="Vehicle speed">
      <span class="widget-kicker">SPEED</span>
      <div class="speed-readout">
        <output ref={value} class="speed-value" aria-label="Vehicle speed in kilometres per hour">
          —
        </output>
        <span class="speed-unit">KM/H</span>
      </div>
    </section>
  );
}
