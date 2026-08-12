import type { WidgetInstance } from "@opencarpanel/widget-sdk";

import type { ConnectionView } from "../connection/client";
import type { TelemetryRenderLoop } from "../telemetry/render-loop";
import type { StatusMode } from "./game-profile";
import { GearWidget } from "../widgets/gear";
import { SpeedWidget } from "../widgets/speed";
import { TachometerWidget } from "../widgets/tachometer";
import { StatusRail } from "./status-rail";

export interface WidgetViewProps {
  readonly widget: WidgetInstance;
  readonly loop: TelemetryRenderLoop;
  readonly connection: ConnectionView;
  readonly statusMode: StatusMode;
}

export function WidgetView({ widget, loop, connection, statusMode }: WidgetViewProps) {
  switch (widget.componentType) {
    case "core.gear":
      return <GearWidget loop={loop} />;
    case "core.speed":
      return <SpeedWidget loop={loop} />;
    case "core.tachometer":
      return <TachometerWidget loop={loop} />;
    case "core.status":
      return <StatusRail loop={loop} connection={connection} mode={statusMode} />;
    default:
      return <div class="unknown-widget">不支持的组件：{widget.componentType}</div>;
  }
}
