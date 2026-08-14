import type {
  BuiltinGameId,
  WidgetManifest,
  WidgetType,
} from "@opencarpanel/widget-sdk";

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

const COMMON_WIDGET_TYPES: readonly WidgetType[] = [
  gearManifest.type,
  speedManifest.type,
  tachometerManifest.type,
  statusManifest.type,
];

const GAME_WIDGET_TYPES: Readonly<Record<BuiltinGameId, readonly WidgetType[]>> = {
  "f1-24": [...COMMON_WIDGET_TYPES, raceManifest.type, tyresManifest.type],
  "f1-25": [...COMMON_WIDGET_TYPES, raceManifest.type, tyresManifest.type],
  ets2: [...COMMON_WIDGET_TYPES, routeManifest.type],
  ats: [...COMMON_WIDGET_TYPES, routeManifest.type],
};

export function builtinWidgetManifestsForGame(gameId: BuiltinGameId) {
  const supportedTypes = GAME_WIDGET_TYPES[gameId];
  return BUILTIN_WIDGET_MANIFESTS.filter((manifest) =>
    supportedTypes.includes(manifest.type),
  );
}

export function builtinWidgetManifest(type: WidgetType): WidgetManifest<object> | undefined {
  return BUILTIN_WIDGET_MANIFESTS.find((manifest) => manifest.type === type);
}
