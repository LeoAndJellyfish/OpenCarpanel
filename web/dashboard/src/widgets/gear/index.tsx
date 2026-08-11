import { useEffect, useRef } from "preact/hooks";

import type { TelemetryRenderLoop } from "../../telemetry/render-loop";
import { formatGear } from "./format";

export { gearManifest } from "./manifest";

export interface GearWidgetProps {
  readonly loop: TelemetryRenderLoop;
}

export function GearWidget({ loop }: GearWidgetProps) {
  const value = useRef<HTMLOutputElement>(null);

  useEffect(
    () =>
      loop.bind(["vehicle.gear"], (store, nowMs) => {
        if (value.current) {
          value.current.textContent = formatGear(store.read("vehicle.gear", nowMs));
        }
      }),
    [loop],
  );

  return (
    <section class="dashboard-widget gear-widget" aria-label="Current gear">
      <span class="widget-kicker">GEAR</span>
      <output ref={value} class="gear-value" aria-label="Current gear">
        –
      </output>
    </section>
  );
}
