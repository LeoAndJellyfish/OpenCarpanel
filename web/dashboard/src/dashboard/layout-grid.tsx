import type {
  BreakpointName,
  GridPlacement,
  LayoutDocument,
  WidgetInstance,
} from "@opensimdash/widget-sdk";
import type { ComponentChildren, Ref } from "preact";

import type { ConnectionView } from "../connection/client";
import type { TelemetryRenderLoop } from "../telemetry/render-loop";
import type { StatusMode } from "./game-profile";
import { WidgetView } from "./widget-view";

export interface LayoutItemContext {
  readonly widget: WidgetInstance;
  readonly placement: GridPlacement;
  readonly className: string;
  readonly content: ComponentChildren;
}

export interface LayoutGridProps {
  readonly layout: LayoutDocument;
  readonly breakpoint: BreakpointName;
  readonly loop: TelemetryRenderLoop;
  readonly connection: ConnectionView;
  readonly statusMode?: StatusMode;
  readonly renderItem?: (context: LayoutItemContext) => ComponentChildren;
  readonly gridRef?: Ref<HTMLElement>;
}

export function LayoutGrid({
  layout,
  breakpoint,
  loop,
  connection,
  statusMode = "generic",
  renderItem,
  gridRef,
}: LayoutGridProps) {
  return (
    <section
      {...(gridRef === undefined ? {} : { ref: gridRef })}
      class="dashboard-layout-grid"
      data-breakpoint={breakpoint}
    >
      {layout.widgets.map((widget) => {
        const placement = widget.placements[breakpoint];
        if (!placement) {
          return null;
        }
        const className = placementClassName(placement);
        const content = (
          <WidgetView
            widget={widget}
            loop={loop}
            connection={connection}
            statusMode={statusMode}
          />
        );
        return renderItem ? (
          renderItem({ widget, placement, className, content })
        ) : (
          <div key={widget.instanceId} class={className} data-component={widget.componentType}>
            {content}
          </div>
        );
      })}
    </section>
  );
}

export function placementClassName(placement: GridPlacement): string {
  return `layout-widget-frame grid-x-${placement.x} grid-y-${placement.y} grid-w-${placement.width} grid-h-${placement.height}`;
}
