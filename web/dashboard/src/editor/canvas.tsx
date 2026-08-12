import {
  BREAKPOINT_GRIDS,
  type BreakpointName,
  type GridPlacement,
  type LayoutDocument,
  type WidgetInstance,
} from "@opencarpanel/widget-sdk";
import type { JSX } from "preact";
import { useEffect, useRef, useState } from "preact/hooks";

import type { ConnectionView } from "../connection/client";
import { LayoutGrid } from "../dashboard/layout-grid";
import type { StatusMode } from "../dashboard/game-profile";
import type { TelemetryRenderLoop } from "../telemetry/render-loop";
import { builtinWidgetManifest } from "../widgets/catalog";
import { movePlacement, resizePlacement, updatePlacement } from "./grid";

type GestureMode = "move" | "resize";

export interface EditorCanvasProps {
  readonly layout: LayoutDocument;
  readonly breakpoint: BreakpointName;
  readonly loop: TelemetryRenderLoop;
  readonly connection: ConnectionView;
  readonly statusMode: StatusMode;
  readonly selectedId: string | undefined;
  readonly onSelect: (instanceId: string) => void;
  readonly onCommit: (layout: LayoutDocument) => void;
}

export function EditorCanvas({
  layout,
  breakpoint,
  loop,
  connection,
  statusMode,
  selectedId,
  onSelect,
  onCommit,
}: EditorCanvasProps) {
  const gridRef = useRef<HTMLElement>(null);
  const [preview, setPreview] = useState<LayoutDocument>();
  const previewRef = useRef<LayoutDocument>();
  const cleanupGesture = useRef<() => void>();
  const displayed = preview ?? layout;

  useEffect(() => {
    setPreview(undefined);
    previewRef.current = undefined;
  }, [layout, breakpoint]);

  useEffect(
    () => () => {
      cleanupGesture.current?.();
    },
    [],
  );

  const keyboardEdit = (
    event: JSX.TargetedKeyboardEvent<HTMLButtonElement>,
    widget: WidgetInstance,
    placement: GridPlacement,
    mode: GestureMode,
  ) => {
    const direction = keyboardDelta(event.key);
    if (!direction) {
      return;
    }
    event.preventDefault();
    const next = editPlacement(layout, widget, placement, breakpoint, mode, direction);
    if (next !== layout) {
      onCommit(next);
    }
  };

  const startGesture = (
    event: JSX.TargetedPointerEvent<HTMLButtonElement>,
    widget: WidgetInstance,
    placement: GridPlacement,
    mode: GestureMode,
  ) => {
    if (event.button !== 0) {
      return;
    }
    const gridElement = gridRef.current;
    if (!gridElement) {
      return;
    }
    event.preventDefault();
    event.stopPropagation();
    onSelect(widget.instanceId);
    cleanupGesture.current?.();

    const bounds = gridElement.getBoundingClientRect();
    const grid = BREAKPOINT_GRIDS[breakpoint];
    const startX = event.clientX;
    const startY = event.clientY;
    const handleMove = (pointerEvent: PointerEvent) => {
      pointerEvent.preventDefault();
      const delta = {
        columns: (pointerEvent.clientX - startX) / Math.max(1, bounds.width / grid.columns),
        rows: (pointerEvent.clientY - startY) / Math.max(1, bounds.height / grid.rows),
      };
      const next = editPlacement(layout, widget, placement, breakpoint, mode, delta);
      previewRef.current = next;
      setPreview(next);
    };
    const finish = () => {
      const next = previewRef.current;
      cleanupGesture.current?.();
      setPreview(undefined);
      previewRef.current = undefined;
      if (next && next !== layout) {
        onCommit(next);
      }
    };
    const cleanup = () => {
      window.removeEventListener("pointermove", handleMove);
      window.removeEventListener("pointerup", finish);
      window.removeEventListener("pointercancel", finish);
      cleanupGesture.current = undefined;
    };
    cleanupGesture.current = cleanup;
    window.addEventListener("pointermove", handleMove, { passive: false });
    window.addEventListener("pointerup", finish, { once: true });
    window.addEventListener("pointercancel", finish, { once: true });
  };

  return (
    <LayoutGrid
      gridRef={gridRef}
      layout={displayed}
      breakpoint={breakpoint}
      loop={loop}
      connection={connection}
      statusMode={statusMode}
      renderItem={({ widget, placement, className, content }) => {
        const manifest = builtinWidgetManifest(widget.componentType);
        const label = manifest?.displayName ?? widget.componentType;
        return (
          <div
            key={widget.instanceId}
            class={`${className} editor-layout-item${selectedId === widget.instanceId ? " is-selected" : ""}`}
            data-component={widget.componentType}
          >
            <button
              class="editor-move-handle"
              type="button"
              aria-label={`移动 ${label}`}
              onClick={() => onSelect(widget.instanceId)}
              onKeyDown={(event) => keyboardEdit(event, widget, placement, "move")}
              onPointerDown={(event) => startGesture(event, widget, placement, "move")}
            >
              <span>{label}</span>
              <span aria-hidden="true">↕</span>
            </button>
            <div class="editor-widget-content" aria-hidden="true">
              {content}
            </div>
            <button
              class="editor-resize-handle"
              type="button"
              aria-label={`调整 ${label} 大小`}
              onKeyDown={(event) => keyboardEdit(event, widget, placement, "resize")}
              onPointerDown={(event) => startGesture(event, widget, placement, "resize")}
            >
              <span aria-hidden="true">↘</span>
            </button>
          </div>
        );
      }}
    />
  );
}

function editPlacement(
  layout: LayoutDocument,
  widget: WidgetInstance,
  placement: GridPlacement,
  breakpoint: BreakpointName,
  mode: GestureMode,
  delta: { readonly columns: number; readonly rows: number },
): LayoutDocument {
  const occupied = layout.widgets.flatMap((other) => {
    const otherPlacement = other.placements[breakpoint];
    return other.instanceId !== widget.instanceId && otherPlacement ? [otherPlacement] : [];
  });
  const grid = BREAKPOINT_GRIDS[breakpoint];
  const manifest = builtinWidgetManifest(widget.componentType);
  const next =
    mode === "move"
      ? movePlacement(placement, delta, grid, occupied)
      : resizePlacement(
          placement,
          delta,
          grid,
          manifest?.minimumSize ?? { columns: 1, rows: 1 },
          occupied,
        );
  return next === placement ? layout : updatePlacement(layout, widget.instanceId, breakpoint, next);
}

function keyboardDelta(key: string): { columns: number; rows: number } | undefined {
  switch (key) {
    case "ArrowLeft":
      return { columns: -1, rows: 0 };
    case "ArrowRight":
      return { columns: 1, rows: 0 };
    case "ArrowUp":
      return { columns: 0, rows: -1 };
    case "ArrowDown":
      return { columns: 0, rows: 1 };
    default:
      return undefined;
  }
}
