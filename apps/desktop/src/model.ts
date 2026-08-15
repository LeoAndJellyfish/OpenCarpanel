import type { HostDiagnostics } from "./desktop-api";

export type GameId = string;

export interface GameProfile {
  id: GameId;
  shortLabel: string;
  label: string;
  accent: string;
  family: "formula" | "truck" | "neutral";
}

const WAITING_PROFILE: GameProfile = {
  id: "waiting",
  shortLabel: "AUTO",
  label: "等待游戏遥测",
  accent: "#aab4af",
  family: "neutral",
};

export function gameProfile(diagnostics: HostDiagnostics): GameProfile {
  const candidate =
    diagnostics.activeAdapter ??
    (diagnostics.adapterSelection === "auto" ? "waiting" : diagnostics.adapterSelection);
  const adapter = diagnostics.supportedAdapters.find((item) => item.id === candidate);
  if (!adapter) {
    return WAITING_PROFILE;
  }
  const presentation = adapter.plugin.presentation;
  return {
    id: adapter.id,
    shortLabel: presentation.shortName,
    label: adapter.plugin.name,
    accent: presentation.theme.accent,
    family: presentation.family === "generic" ? "neutral" : presentation.family,
  };
}

export function telemetryIsLive(diagnostics: HostDiagnostics): boolean {
  const age = diagnostics.telemetry.lastPacketAgeMs;
  return diagnostics.activeAdapter !== null && age !== null && age < 2_500;
}

export function formatAge(milliseconds: number | null): string {
  if (milliseconds === null) {
    return "—";
  }
  if (milliseconds < 1_000) {
    return `${Math.round(milliseconds)} ms`;
  }
  if (milliseconds < 60_000) {
    return `${(milliseconds / 1_000).toFixed(1)} s`;
  }
  return `${Math.floor(milliseconds / 60_000)} min`;
}

export function formatUptime(milliseconds: number): string {
  const totalMinutes = Math.floor(milliseconds / 60_000);
  const days = Math.floor(totalMinutes / 1_440);
  const hours = Math.floor((totalMinutes % 1_440) / 60);
  const minutes = totalMinutes % 60;
  if (days > 0) {
    return `${days}d ${hours}h`;
  }
  if (hours > 0) {
    return `${hours}h ${minutes}m`;
  }
  return `${minutes}m`;
}

export function compactNumber(value: number): string {
  return new Intl.NumberFormat("zh-CN", {
    notation: value >= 10_000 ? "compact" : "standard",
    maximumFractionDigits: 1,
  }).format(value);
}

export function formatDeviceTime(unixMilliseconds: number): string {
  return new Intl.DateTimeFormat("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  }).format(new Date(unixMilliseconds));
}
