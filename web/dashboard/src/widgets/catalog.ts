import type { WidgetManifest, WidgetType } from "@opencarpanel/widget-sdk";

import { statusManifest } from "../dashboard/status-manifest";
import { gearManifest } from "./gear";
import { speedManifest } from "./speed";
import { tachometerManifest } from "./tachometer";

export const BUILTIN_WIDGET_MANIFESTS = [
  gearManifest,
  speedManifest,
  tachometerManifest,
  statusManifest,
] as const;

export function builtinWidgetManifest(type: WidgetType): WidgetManifest<object> | undefined {
  return BUILTIN_WIDGET_MANIFESTS.find((manifest) => manifest.type === type);
}
