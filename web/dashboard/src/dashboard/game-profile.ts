import {
  BUILTIN_GAME_PLUGINS,
  DEFAULT_LAYOUT,
  gameDefaultLayout,
  type GamePluginMetadata,
  type LayoutDocument,
} from "@opensimdash/widget-sdk";

export type GameFamily = "formula" | "truck" | "neutral";
export type StatusMode = "drs" | "scs" | "generic";

export interface GamePresentation {
  readonly id: string;
  readonly label: string;
  readonly detail: string;
  readonly family: GameFamily;
  readonly statusMode: StatusMode;
  readonly layoutId: string;
  readonly defaultLayout: LayoutDocument;
  readonly plugin: GamePluginMetadata | undefined;
}

function presentationForPlugin(plugin: GamePluginMetadata): GamePresentation {
  const defaultLayout = gameDefaultLayout(plugin);
  return {
    id: plugin.id,
    label: plugin.name.toUpperCase(),
    detail: plugin.presentation.detail.toUpperCase(),
    family: plugin.presentation.family === "generic" ? "neutral" : plugin.presentation.family,
    statusMode: plugin.presentation.statusMode,
    layoutId: defaultLayout.id,
    defaultLayout,
    plugin,
  };
}

export function gamePresentations(
  plugins: readonly GamePluginMetadata[],
): readonly GamePresentation[] {
  return plugins.map(presentationForPlugin);
}

export const SUPPORTED_GAME_PRESENTATIONS = gamePresentations(BUILTIN_GAME_PLUGINS);

const UNKNOWN_GAME_PRESENTATION: GamePresentation = {
  id: "unknown",
  label: "OPENSIMDASH",
  detail: "WAITING FOR GAME TELEMETRY",
  family: "neutral",
  statusMode: "generic",
  layoutId: "default",
  defaultLayout: DEFAULT_LAYOUT,
  plugin: undefined,
};

export function gamePresentation(
  gameId: string | null | undefined,
  plugins: readonly GamePluginMetadata[] = BUILTIN_GAME_PLUGINS,
): GamePresentation {
  const plugin = plugins.find((candidate) => candidate.id === gameId);
  return plugin ? presentationForPlugin(plugin) : UNKNOWN_GAME_PRESENTATION;
}

export function isGamePluginId(
  value: string | null | undefined,
  plugins: readonly GamePluginMetadata[],
): value is string {
  return plugins.some((plugin) => plugin.id === value);
}

export function isBuiltinGameId(value: string | null | undefined): boolean {
  return BUILTIN_GAME_PLUGINS.some((plugin) => plugin.id === value);
}
