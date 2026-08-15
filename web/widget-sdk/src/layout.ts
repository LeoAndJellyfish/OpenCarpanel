import type {
  GridPlacement as GeneratedGridPlacement,
  LayoutDocument as GeneratedLayoutDocument,
  ThemeSettings as GeneratedThemeSettings,
  WidgetInstance as GeneratedWidgetInstance,
} from "./generated/layout-document";
import { BUILTIN_GAME_PLUGINS } from "./generated/builtin-game-plugins";
import type { GamePluginMetadata } from "./generated/server-message";
import type { WidgetType } from "./manifest";

export const LAYOUT_SCHEMA_VERSION = 1 as const;

export const BREAKPOINT_NAMES = [
  "phonePortrait",
  "phoneLandscape",
  "tablet",
  "desktop",
] as const;

export type BreakpointName = (typeof BREAKPOINT_NAMES)[number];

export type BuiltinGameId = (typeof BUILTIN_GAME_PLUGINS)[number]["id"];

export const BUILTIN_GAME_IDS = BUILTIN_GAME_PLUGINS.map(
  (plugin) => plugin.id,
) as readonly BuiltinGameId[];

export interface BreakpointGrid {
  readonly columns: number;
  readonly rows: number;
}

export const BREAKPOINT_GRIDS: Readonly<Record<BreakpointName, BreakpointGrid>> = {
  phonePortrait: { columns: 12, rows: 18 },
  phoneLandscape: { columns: 12, rows: 10 },
  tablet: { columns: 12, rows: 12 },
  desktop: { columns: 12, rows: 12 },
};

export type GridPlacement = Pick<GeneratedGridPlacement, "x" | "y" | "width" | "height">;

export interface ThemeSettings
  extends Required<
    Pick<GeneratedThemeSettings, "background" | "foreground" | "accent" | "warning">
  > {}

export interface WidgetInstance
  extends Pick<GeneratedWidgetInstance, "instanceId"> {
  readonly componentType: WidgetType;
  readonly placements: Partial<Record<BreakpointName, GridPlacement>>;
  readonly settings: Readonly<Record<string, unknown>>;
}

export interface LayoutDocument
  extends Pick<GeneratedLayoutDocument, "id" | "name" | "revision"> {
  readonly schemaVersion: typeof LAYOUT_SCHEMA_VERSION;
  readonly widgets: readonly WidgetInstance[];
  readonly theme: ThemeSettings;
}

type PlacementSet<Names extends string> = Readonly<
  Record<Names, Record<BreakpointName, GridPlacement>>
>;

type FormulaPlacements = PlacementSet<
  "tachometer" | "gear" | "speed" | "status" | "race" | "tyres"
>;
type TruckPlacements = PlacementSet<
  "tachometer" | "gear" | "speed" | "status" | "route"
>;
type DefaultPlacements = FormulaPlacements | TruckPlacements;

const FORMULA_PLACEMENTS: FormulaPlacements = {
  tachometer: {
    phonePortrait: { x: 0, y: 0, width: 12, height: 3 },
    phoneLandscape: { x: 0, y: 0, width: 12, height: 3 },
    tablet: { x: 0, y: 0, width: 12, height: 3 },
    desktop: { x: 0, y: 0, width: 12, height: 3 },
  },
  gear: {
    phonePortrait: { x: 2, y: 3, width: 8, height: 9 },
    phoneLandscape: { x: 4, y: 3, width: 4, height: 5 },
    tablet: { x: 4, y: 3, width: 4, height: 6 },
    desktop: { x: 4, y: 3, width: 4, height: 6 },
  },
  speed: {
    phonePortrait: { x: 0, y: 12, width: 5, height: 3 },
    phoneLandscape: { x: 0, y: 3, width: 4, height: 5 },
    tablet: { x: 0, y: 3, width: 4, height: 6 },
    desktop: { x: 0, y: 3, width: 4, height: 6 },
  },
  status: {
    phonePortrait: { x: 6, y: 12, width: 6, height: 3 },
    phoneLandscape: { x: 8, y: 3, width: 4, height: 5 },
    tablet: { x: 8, y: 3, width: 4, height: 6 },
    desktop: { x: 8, y: 3, width: 4, height: 6 },
  },
  race: {
    phonePortrait: { x: 0, y: 15, width: 7, height: 3 },
    phoneLandscape: { x: 0, y: 8, width: 7, height: 2 },
    tablet: { x: 0, y: 9, width: 7, height: 3 },
    desktop: { x: 0, y: 9, width: 7, height: 3 },
  },
  tyres: {
    phonePortrait: { x: 7, y: 15, width: 5, height: 3 },
    phoneLandscape: { x: 7, y: 8, width: 5, height: 2 },
    tablet: { x: 7, y: 9, width: 5, height: 3 },
    desktop: { x: 7, y: 9, width: 5, height: 3 },
  },
};

const TRUCK_PLACEMENTS: TruckPlacements = {
  speed: {
    phonePortrait: { x: 0, y: 0, width: 12, height: 7 },
    phoneLandscape: { x: 0, y: 0, width: 7, height: 7 },
    tablet: { x: 0, y: 0, width: 7, height: 8 },
    desktop: { x: 0, y: 0, width: 7, height: 8 },
  },
  gear: {
    phonePortrait: { x: 0, y: 7, width: 5, height: 5 },
    phoneLandscape: { x: 7, y: 0, width: 5, height: 4 },
    tablet: { x: 7, y: 0, width: 5, height: 4 },
    desktop: { x: 7, y: 0, width: 5, height: 4 },
  },
  tachometer: {
    phonePortrait: { x: 5, y: 7, width: 7, height: 3 },
    phoneLandscape: { x: 0, y: 7, width: 7, height: 3 },
    tablet: { x: 0, y: 8, width: 7, height: 4 },
    desktop: { x: 0, y: 8, width: 7, height: 4 },
  },
  status: {
    phonePortrait: { x: 5, y: 10, width: 7, height: 2 },
    phoneLandscape: { x: 7, y: 4, width: 5, height: 2 },
    tablet: { x: 7, y: 4, width: 5, height: 2 },
    desktop: { x: 7, y: 4, width: 5, height: 2 },
  },
  route: {
    phonePortrait: { x: 0, y: 12, width: 12, height: 6 },
    phoneLandscape: { x: 7, y: 6, width: 5, height: 4 },
    tablet: { x: 7, y: 6, width: 5, height: 6 },
    desktop: { x: 7, y: 6, width: 5, height: 6 },
  },
};

function builtInLayout(
  id: string,
  name: string,
  placements: DefaultPlacements,
  theme: ThemeSettings,
  fallbackRpmMax: number,
  family: "formula" | "truck" | "generic",
  supportedWidgets: readonly string[],
): LayoutDocument {
  const supplementalWidgets: WidgetInstance[] = family === "formula"
    ? [
        {
          instanceId: "race",
          componentType: "core.race",
          placements: (placements as FormulaPlacements).race,
          settings: {},
        },
        {
          instanceId: "tyres",
          componentType: "core.tyres",
          placements: (placements as FormulaPlacements).tyres,
          settings: {},
        },
      ]
    : family === "truck" ? [
        {
          instanceId: "route",
          componentType: "core.route",
          placements: (placements as TruckPlacements).route,
          settings: {},
        },
      ] : [];
  const widgets: WidgetInstance[] = [
    {
      instanceId: "tachometer",
      componentType: "core.tachometer",
      placements: placements.tachometer,
      settings: { fallbackRpmMax },
    },
    {
      instanceId: "gear",
      componentType: "core.gear",
      placements: placements.gear,
      settings: {},
    },
    {
      instanceId: "speed",
      componentType: "core.speed",
      placements: placements.speed,
      settings: { unit: "km/h" },
    },
    {
      instanceId: "status",
      componentType: "core.status",
      placements: placements.status,
      settings: {},
    },
    ...supplementalWidgets,
  ];
  return {
    schemaVersion: LAYOUT_SCHEMA_VERSION,
    revision: 0,
    id,
    name,
    widgets: widgets.filter((widget) => supportedWidgets.includes(widget.componentType)),
    theme,
  };
}

export function gameDefaultLayout(plugin: GamePluginMetadata): LayoutDocument {
  const preset = plugin.presentation.layoutPreset;
  return builtInLayout(
    `game-${plugin.id}`,
    `${plugin.presentation.shortName} Default`,
    preset === "truck" ? TRUCK_PLACEMENTS : FORMULA_PLACEMENTS,
    { ...plugin.presentation.theme },
    plugin.presentation.fallbackRpmMax,
    preset,
    plugin.presentation.widgets,
  );
}

const FALLBACK_PLUGIN = BUILTIN_GAME_PLUGINS.find(
  (plugin) => plugin.presentation.layoutPreset === "formula",
) ?? BUILTIN_GAME_PLUGINS[0];

export const DEFAULT_LAYOUT: LayoutDocument = FALLBACK_PLUGIN
  ? {
      ...gameDefaultLayout(FALLBACK_PLUGIN),
      id: "default",
      name: "OpenCarpanel Default",
    }
  : {
      schemaVersion: LAYOUT_SCHEMA_VERSION,
      revision: 0,
      id: "default",
      name: "OpenCarpanel Default",
      widgets: [],
      theme: {
        background: "#07090c",
        foreground: "#f2f0e9",
        accent: "#d9ff43",
        warning: "#ff4b3e",
      },
    };

export const GAME_DEFAULT_LAYOUTS = Object.fromEntries(
  BUILTIN_GAME_PLUGINS.map((plugin) => [plugin.id, gameDefaultLayout(plugin)]),
) as Readonly<Record<BuiltinGameId, LayoutDocument>>;

export function cloneLayout(layout: LayoutDocument): LayoutDocument {
  return JSON.parse(JSON.stringify(layout)) as LayoutDocument;
}

export class LayoutParseError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "LayoutParseError";
  }
}

export function parseLayoutDocument(value: unknown): LayoutDocument {
  const document = objectValue(value, "layout");
  if (document.schemaVersion !== LAYOUT_SCHEMA_VERSION) {
    throw new LayoutParseError("unsupported layout schema version");
  }
  const revision = safeInteger(document.revision, "layout revision", 0);
  const id = safeIdentifier(document.id, "layout id");
  const name = boundedString(document.name, "layout name", 1, 256);
  const themeValue = objectValue(document.theme, "layout theme");
  const theme: ThemeSettings = {
    background: color(themeValue.background, "theme background"),
    foreground: color(themeValue.foreground, "theme foreground"),
    accent: color(themeValue.accent, "theme accent"),
    warning: color(themeValue.warning, "theme warning"),
  };
  if (!Array.isArray(document.widgets) || document.widgets.length > 64) {
    throw new LayoutParseError("layout widgets must be an array of at most 64 entries");
  }

  const instanceIds = new Set<string>();
  const widgets = document.widgets.map((entry, index) => {
    const widget = objectValue(entry, `widget ${index}`);
    const instanceId = safeIdentifier(widget.instanceId, `widget ${index} instance id`);
    if (instanceIds.has(instanceId)) {
      throw new LayoutParseError(`duplicate widget instance ${instanceId}`);
    }
    instanceIds.add(instanceId);
    const componentType = componentIdentifier(widget.componentType, `widget ${index} type`);
    const rawPlacements = objectValue(widget.placements, `widget ${index} placements`);
    const placements: Partial<Record<BreakpointName, GridPlacement>> = {};
    for (const breakpoint of BREAKPOINT_NAMES) {
      const rawPlacement = rawPlacements[breakpoint];
      if (rawPlacement === undefined) {
        continue;
      }
      const placement = objectValue(rawPlacement, `${breakpoint} placement`);
      const x = safeInteger(placement.x, "placement x", 0);
      const y = safeInteger(placement.y, "placement y", 0);
      const width = safeInteger(placement.width, "placement width", 1);
      const height = safeInteger(placement.height, "placement height", 1);
      if (x + width > 24 || y + height > 1_000) {
        throw new LayoutParseError("widget placement is outside the supported canvas");
      }
      placements[breakpoint] = { x, y, width, height };
    }
    const settings = objectValue(widget.settings, `widget ${index} settings`);
    return { instanceId, componentType, placements, settings } satisfies WidgetInstance;
  });

  return {
    schemaVersion: LAYOUT_SCHEMA_VERSION,
    revision,
    id,
    name,
    widgets,
    theme,
  };
}

function objectValue(value: unknown, field: string): Record<string, unknown> {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    throw new LayoutParseError(`${field} must be an object`);
  }
  return value as Record<string, unknown>;
}

function safeInteger(value: unknown, field: string, minimum: number): number {
  if (!Number.isSafeInteger(value) || (value as number) < minimum) {
    throw new LayoutParseError(`${field} must be a safe integer of at least ${minimum}`);
  }
  return value as number;
}

function boundedString(
  value: unknown,
  field: string,
  minimumLength: number,
  maximumLength: number,
): string {
  if (
    typeof value !== "string" ||
    value.length < minimumLength ||
    utf8ByteLength(value) > maximumLength
  ) {
    throw new LayoutParseError(`${field} has an invalid length`);
  }
  return value;
}

function safeIdentifier(value: unknown, field: string): string {
  const text = boundedString(value, field, 1, 96);
  if (!/^[a-z0-9](?:[a-z0-9-]*[a-z0-9])?$/.test(text) || text.includes("--")) {
    throw new LayoutParseError(`${field} is invalid`);
  }
  return text;
}

function componentIdentifier(value: unknown, field: string): WidgetType {
  const text = boundedString(value, field, 3, 96);
  if (!/^[a-z0-9][a-z0-9-]*(?:\.[a-z0-9][a-z0-9-]*)+$/.test(text)) {
    throw new LayoutParseError(`${field} is invalid`);
  }
  return text as WidgetType;
}

function color(value: unknown, field: string): string {
  if (
    typeof value !== "string" ||
    !/^#(?:[0-9a-fA-F]{3,4}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})$/.test(value)
  ) {
    throw new LayoutParseError(`${field} must be a hexadecimal color`);
  }
  return value;
}

function utf8ByteLength(value: string): number {
  let length = 0;
  for (const character of value) {
    const codePoint = character.codePointAt(0) ?? 0;
    length += codePoint <= 0x7f ? 1 : codePoint <= 0x7ff ? 2 : codePoint <= 0xffff ? 3 : 4;
  }
  return length;
}
