import type { ThemeSettings } from "@opencarpanel/widget-sdk";

export const THEME_PRESETS = [
  {
    id: "signal",
    name: "赛道信号",
    theme: {
      background: "#07090c",
      foreground: "#f2f0e9",
      accent: "#d9ff43",
      warning: "#ff4b3e",
    },
  },
  {
    id: "cyan",
    name: "冷却青",
    theme: {
      background: "#061015",
      foreground: "#eefcff",
      accent: "#42e8ff",
      warning: "#ff5e6c",
    },
  },
  {
    id: "amber",
    name: "维修区琥珀",
    theme: {
      background: "#0e0b08",
      foreground: "#fff5e5",
      accent: "#ffbd45",
      warning: "#ff4b3e",
    },
  },
  {
    id: "mono",
    name: "高对比单色",
    theme: {
      background: "#050505",
      foreground: "#ffffff",
      accent: "#ffffff",
      warning: "#ff4b3e",
    },
  },
  {
    id: "road",
    name: "州际公路",
    theme: {
      background: "#080d10",
      foreground: "#f5f0e6",
      accent: "#ff6a3d",
      warning: "#ffcf54",
    },
  },
] as const satisfies readonly {
  readonly id: string;
  readonly name: string;
  readonly theme: ThemeSettings;
}[];

export type ThemePresetId = (typeof THEME_PRESETS)[number]["id"];

export function themePresetId(theme: ThemeSettings): ThemePresetId {
  const signature = themeSignature(theme);
  return THEME_PRESETS.find((preset) => themeSignature(preset.theme) === signature)?.id ?? "signal";
}

export function dashboardThemeStyle(theme: ThemeSettings): string {
  return `--surface-0:${theme.background};--ink:${theme.foreground};--signal:${theme.accent};--redline:${theme.warning}`;
}

function themeSignature(theme: ThemeSettings): string {
  return `${theme.background}|${theme.foreground}|${theme.accent}|${theme.warning}`.toLowerCase();
}
