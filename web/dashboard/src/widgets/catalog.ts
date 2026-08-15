import type {
  GamePluginMetadata,
  WidgetManifest,
  WidgetType,
} from "@opencarpanel/widget-sdk";
import { BUILTIN_GAME_PLUGINS } from "@opencarpanel/widget-sdk";

import { statusManifest } from "../dashboard/status-manifest";
import { gearManifest } from "./gear";
import { raceManifest } from "./race";
import { routeManifest } from "./route";
import { speedManifest } from "./speed";
import { tachometerManifest } from "./tachometer";
import { tyresManifest } from "./tyres";

export const BUILTIN_WIDGET_MANIFESTS = [
  gearManifest,
  raceManifest,
  routeManifest,
  speedManifest,
  tachometerManifest,
  tyresManifest,
  statusManifest,
] as const;

export function builtinWidgetManifestsForPlugin(plugin: GamePluginMetadata | undefined) {
  const supportedTypes = new Set(plugin?.presentation.widgets ?? []);
  return BUILTIN_WIDGET_MANIFESTS.filter((manifest) =>
    supportedTypes.has(manifest.type),
  );
}

export function builtinWidgetManifestsForGame(gameId: string) {
  return builtinWidgetManifestsForPlugin(
    BUILTIN_GAME_PLUGINS.find((plugin) => plugin.id === gameId),
  );
}

export function builtinWidgetManifest(type: WidgetType): WidgetManifest<object> | undefined {
  return BUILTIN_WIDGET_MANIFESTS.find((manifest) => manifest.type === type);
}
