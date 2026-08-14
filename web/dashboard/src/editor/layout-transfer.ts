import {
  BREAKPOINT_GRIDS,
  BREAKPOINT_NAMES,
  LayoutParseError,
  parseLayoutDocument,
  type BreakpointName,
  type GridPlacement,
  type LayoutDocument,
  type WidgetInstance,
} from "@opencarpanel/widget-sdk";

import { builtinWidgetManifest } from "../widgets/catalog";
import { placementsOverlap } from "./grid";

export const MAX_LAYOUT_TRANSFER_BYTES = 256 * 1024;

export class LayoutTransferError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "LayoutTransferError";
  }
}

export interface LayoutExport {
  readonly filename: string;
  readonly content: string;
}

export function createLayoutExport(document: LayoutDocument): LayoutExport {
  return {
    filename: `opencarpanel-${document.id}.json`,
    content: `${JSON.stringify(document, undefined, 2)}\n`,
  };
}

export function importLayoutText(
  text: string,
  destination: Pick<LayoutDocument, "id" | "revision">,
): LayoutDocument {
  if (utf8ByteLength(text) > MAX_LAYOUT_TRANSFER_BYTES) {
    throw new LayoutTransferError("布局文件超过 256 KB 限制。");
  }

  const withoutBom = text.charCodeAt(0) === 0xfeff ? text.slice(1) : text;
  let raw: unknown;
  try {
    raw = JSON.parse(withoutBom) as unknown;
  } catch {
    throw new LayoutTransferError("布局文件不是有效的 JSON。");
  }

  let parsed: LayoutDocument;
  try {
    parsed = parseLayoutDocument(raw);
  } catch (error: unknown) {
    throw new LayoutTransferError(
      error instanceof LayoutParseError ? error.message : "布局结构无效。",
    );
  }
  validateBuiltinLayout(parsed);

  // Imports replace only the editable content. The destination identity and
  // optimistic-concurrency revision always come from the currently loaded Host document.
  return {
    ...parsed,
    id: destination.id,
    revision: destination.revision,
  };
}

function validateBuiltinLayout(document: LayoutDocument): void {
  for (const widget of document.widgets) {
    const manifest = builtinWidgetManifest(widget.componentType);
    if (!manifest) {
      throw new LayoutTransferError(`不支持组件 ${widget.componentType}。`);
    }
    validateSettings(widget);
    for (const breakpoint of BREAKPOINT_NAMES) {
      const placement = widget.placements[breakpoint];
      if (!placement) {
        continue;
      }
      const grid = BREAKPOINT_GRIDS[breakpoint];
      if (
        placement.width < manifest.minimumSize.columns ||
        placement.height < manifest.minimumSize.rows ||
        placement.x + placement.width > grid.columns ||
        placement.y + placement.height > grid.rows
      ) {
        throw new LayoutTransferError(
          `组件 ${widget.instanceId} 超出 ${breakpoint} 网格或小于最小尺寸。`,
        );
      }
    }
  }

  for (const breakpoint of BREAKPOINT_NAMES) {
    validateNoCollisions(document.widgets, breakpoint);
  }
}

function validateNoCollisions(
  widgets: readonly WidgetInstance[],
  breakpoint: BreakpointName,
): void {
  const placed: { readonly id: string; readonly placement: GridPlacement }[] = [];
  for (const widget of widgets) {
    const placement = widget.placements[breakpoint];
    if (!placement) {
      continue;
    }
    const collision = placed.find((other) => placementsOverlap(placement, other.placement));
    if (collision) {
      throw new LayoutTransferError(
        `${breakpoint} 中的组件 ${collision.id} 与 ${widget.instanceId} 重叠。`,
      );
    }
    placed.push({ id: widget.instanceId, placement });
  }
}

function validateSettings(widget: WidgetInstance): void {
  const settings = widget.settings;
  const keys = Object.keys(settings);
  let valid = false;
  switch (widget.componentType) {
    case "core.gear":
    case "core.race":
    case "core.route":
    case "core.status":
    case "core.tyres":
      valid = keys.length === 0;
      break;
    case "core.speed":
      valid =
        keys.every((key) => key === "unit") &&
        (settings.unit === undefined || settings.unit === "km/h");
      break;
    case "core.tachometer": {
      const rpm = settings.fallbackRpmMax;
      valid =
        keys.every((key) => key === "fallbackRpmMax") &&
        (rpm === undefined ||
          (Number.isSafeInteger(rpm) && Number(rpm) >= 1_000 && Number(rpm) <= 30_000));
      break;
    }
  }
  if (!valid) {
    throw new LayoutTransferError(`组件 ${widget.instanceId} 的设置无效。`);
  }
}

function utf8ByteLength(value: string): number {
  return new TextEncoder().encode(value).length;
}
