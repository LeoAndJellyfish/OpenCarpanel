import type {
  BreakpointGrid,
  BreakpointName,
  GridPlacement,
  LayoutDocument,
} from "@opencarpanel/widget-sdk";

export interface GridDelta {
  readonly columns: number;
  readonly rows: number;
}

export interface GridMinimumSize {
  readonly columns: number;
  readonly rows: number;
}

export function movePlacement(
  placement: GridPlacement,
  delta: GridDelta,
  grid: BreakpointGrid,
  occupied: readonly GridPlacement[],
): GridPlacement {
  const next = {
    ...placement,
    x: clamp(
      placement.x + Math.round(delta.columns),
      0,
      Math.max(0, grid.columns - placement.width),
    ),
    y: clamp(
      placement.y + Math.round(delta.rows),
      0,
      Math.max(0, grid.rows - placement.height),
    ),
  };
  return occupied.some((other) => placementsOverlap(next, other)) ? placement : next;
}

export function resizePlacement(
  placement: GridPlacement,
  delta: GridDelta,
  grid: BreakpointGrid,
  minimum: GridMinimumSize,
  occupied: readonly GridPlacement[],
): GridPlacement {
  const next = {
    ...placement,
    width: clamp(
      placement.width + Math.round(delta.columns),
      minimum.columns,
      grid.columns - placement.x,
    ),
    height: clamp(
      placement.height + Math.round(delta.rows),
      minimum.rows,
      grid.rows - placement.y,
    ),
  };
  return occupied.some((other) => placementsOverlap(next, other)) ? placement : next;
}

export function findAvailablePlacement(
  size: GridMinimumSize,
  grid: BreakpointGrid,
  occupied: readonly GridPlacement[],
): GridPlacement | undefined {
  const width = clamp(size.columns, 1, grid.columns);
  const height = clamp(size.rows, 1, grid.rows);
  for (let y = 0; y <= grid.rows - height; y += 1) {
    for (let x = 0; x <= grid.columns - width; x += 1) {
      const candidate = { x, y, width, height };
      if (!occupied.some((other) => placementsOverlap(candidate, other))) {
        return candidate;
      }
    }
  }
  return undefined;
}

export function placementsOverlap(left: GridPlacement, right: GridPlacement): boolean {
  return (
    left.x < right.x + right.width &&
    left.x + left.width > right.x &&
    left.y < right.y + right.height &&
    left.y + left.height > right.y
  );
}

export function updatePlacement(
  layout: LayoutDocument,
  instanceId: string,
  breakpoint: BreakpointName,
  placement: GridPlacement,
): LayoutDocument {
  return {
    ...layout,
    widgets: layout.widgets.map((widget) =>
      widget.instanceId === instanceId
        ? {
            ...widget,
            placements: { ...widget.placements, [breakpoint]: placement },
          }
        : widget,
    ),
  };
}

function clamp(value: number, minimum: number, maximum: number): number {
  return Math.min(maximum, Math.max(minimum, value));
}
