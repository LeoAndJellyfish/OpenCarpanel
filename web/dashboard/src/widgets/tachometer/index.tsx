import { useEffect, useRef } from "preact/hooks";

import type { TelemetryRenderLoop } from "../../telemetry/render-loop";
import { REV_SEGMENT_COUNT, activationRank, activeSegmentCount } from "./segments";

export { tachometerManifest } from "./manifest";

const segments = Array.from({ length: REV_SEGMENT_COUNT }, (_, index) => ({
  index,
  rank: activationRank(index),
}));

export interface TachometerWidgetProps {
  readonly loop: TelemetryRenderLoop;
}

export function TachometerWidget({ loop }: TachometerWidgetProps) {
  const root = useRef<HTMLElement>(null);
  const rpmValue = useRef<HTMLOutputElement>(null);
  const segmentElements = useRef<Array<SVGRectElement | undefined>>([]);
  const lastActiveCount = useRef(-1);

  useEffect(
    () =>
      loop.bind(
        ["vehicle.rpm", "vehicle.rpmMax", "vehicle.revLights"],
        (store, nowMs) => {
          const rpm = store.read("vehicle.rpm", nowMs);
          const rpmMax = store.read("vehicle.rpmMax", nowMs);
          const gameProgress = store.read("vehicle.revLights", nowMs);
          const progress =
            gameProgress ??
            (rpm === undefined ? undefined : rpm / Math.max(1, rpmMax ?? 12_000));
          const activeCount = activeSegmentCount(progress);

          if (rpmValue.current) {
            rpmValue.current.textContent = rpm === undefined ? "NO RPM" : `${Math.round(rpm)} RPM`;
          }
          if (root.current) {
            root.current.dataset.shift = activeCount >= 18 ? "ready" : "building";
          }
          if (activeCount === lastActiveCount.current) {
            return;
          }
          lastActiveCount.current = activeCount;
          for (const segment of segments) {
            const element = segmentElements.current[segment.index];
            if (!element) {
              continue;
            }
            const active = segment.rank < activeCount;
            element.classList.toggle("rev-segment-active", active);
          }
        },
      ),
    [loop],
  );

  return (
    <section ref={root} class="dashboard-widget tachometer-widget" aria-label="Engine rev lights">
      <div class="tachometer-meta">
        <span class="widget-kicker">SHIFT HORIZON</span>
        <output ref={rpmValue} class="rpm-value" aria-label="Engine revolutions per minute">
          NO RPM
        </output>
      </div>
      <svg class="rev-horizon" viewBox="0 0 1000 84" aria-hidden="true">
        {segments.map(({ index, rank }) => {
          const height = 24 + (9.5 - Math.abs(index - 9.5)) * 3.4;
          return (
            <rect
              key={index}
              ref={(element) => {
                segmentElements.current[index] = element ?? undefined;
              }}
              class={rank >= 16 ? "rev-segment rev-segment-redline" : "rev-segment"}
              x={18 + index * 49}
              y={76 - height}
              width="34"
              height={height}
              rx="4"
            />
          );
        })}
      </svg>
    </section>
  );
}
