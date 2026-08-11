import {
  BREAKPOINT_GRIDS,
  BREAKPOINT_NAMES,
  type BreakpointName,
  type GridPlacement,
  type LayoutDocument,
  type WidgetInstance,
  type WidgetManifest,
} from "@opencarpanel/widget-sdk";

import { findAvailablePlacement } from "./grid";

export function addWidget(
  layout: LayoutDocument,
  manifest: WidgetManifest<object>,
): LayoutDocument | undefined {
  const placements = availablePlacements(layout, manifest, undefined);
  if (!placements) {
    return undefined;
  }
  const widget: WidgetInstance = {
    instanceId: nextInstanceId(layout, manifest.type.split(".").at(-1) ?? "widget"),
    componentType: manifest.type,
    placements,
    settings: cloneSettings(manifest.defaultSettings),
  };
  return { ...layout, widgets: [...layout.widgets, widget] };
}

export function duplicateWidget(
  layout: LayoutDocument,
  instanceId: string,
  manifest: WidgetManifest<object>,
): LayoutDocument | undefined {
  const source = layout.widgets.find((widget) => widget.instanceId === instanceId);
  if (!source) {
    return undefined;
  }
  const placements = availablePlacements(layout, manifest, source);
  if (!placements) {
    return undefined;
  }
  const duplicate: WidgetInstance = {
    ...source,
    instanceId: nextInstanceId(layout, source.instanceId),
    placements,
    settings: cloneSettings(source.settings),
  };
  return { ...layout, widgets: [...layout.widgets, duplicate] };
}

export function removeWidget(layout: LayoutDocument, instanceId: string): LayoutDocument {
  return {
    ...layout,
    widgets: layout.widgets.filter((widget) => widget.instanceId !== instanceId),
  };
}

export function updateWidgetSettings(
  layout: LayoutDocument,
  instanceId: string,
  settings: Readonly<Record<string, unknown>>,
): LayoutDocument {
  return {
    ...layout,
    widgets: layout.widgets.map((widget) =>
      widget.instanceId === instanceId ? { ...widget, settings } : widget,
    ),
  };
}

function availablePlacements(
  layout: LayoutDocument,
  manifest: WidgetManifest<object>,
  source: WidgetInstance | undefined,
): Partial<Record<BreakpointName, GridPlacement>> | undefined {
  const placements: Partial<Record<BreakpointName, GridPlacement>> = {};
  for (const breakpoint of BREAKPOINT_NAMES) {
    const occupied = layout.widgets.flatMap((widget) => {
      const placement = widget.placements[breakpoint];
      return placement ? [placement] : [];
    });
    const sourcePlacement = source?.placements[breakpoint];
    const preferred = sourcePlacement
      ? { columns: sourcePlacement.width, rows: sourcePlacement.height }
      : manifest.defaultSize;
    const placement =
      findAvailablePlacement(preferred, BREAKPOINT_GRIDS[breakpoint], occupied) ??
      findAvailablePlacement(manifest.minimumSize, BREAKPOINT_GRIDS[breakpoint], occupied);
    if (!placement) {
      return undefined;
    }
    placements[breakpoint] = placement;
  }
  return placements;
}

function nextInstanceId(layout: LayoutDocument, rawBase: string): string {
  const base = rawBase
    .toLowerCase()
    .replace(/[^a-z0-9-]/g, "-")
    .replace(/-+/g, "-")
    .replace(/^-|-$/g, "") || "widget";
  const known = new Set(layout.widgets.map((widget) => widget.instanceId));
  if (!known.has(base)) {
    return base;
  }
  for (let number = 2; number <= 999; number += 1) {
    const candidate = `${base}-${number}`;
    if (!known.has(candidate)) {
      return candidate;
    }
  }
  return `${base}-copy`;
}

function cloneSettings(settings: Readonly<object>): Readonly<Record<string, unknown>> {
  return JSON.parse(JSON.stringify(settings)) as Record<string, unknown>;
}
