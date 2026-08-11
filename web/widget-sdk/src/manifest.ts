import type { TelemetryField } from "./generated/server-message";

export const WIDGET_MANIFEST_VERSION = 1 as const;

export type WidgetType = `${string}.${string}`;

export interface WidgetGridSize {
  readonly columns: number;
  readonly rows: number;
}

export interface WidgetManifest<Settings extends object = Record<string, never>> {
  readonly schemaVersion: typeof WIDGET_MANIFEST_VERSION;
  readonly type: WidgetType;
  readonly displayName: string;
  readonly description: string;
  readonly fields: readonly TelemetryField[];
  readonly minimumSize: WidgetGridSize;
  readonly defaultSize: WidgetGridSize;
  readonly defaultSettings: Readonly<Settings>;
}

export function defineWidgetManifest<const Settings extends object>(
  manifest: WidgetManifest<Settings>,
): WidgetManifest<Settings> {
  return manifest;
}
