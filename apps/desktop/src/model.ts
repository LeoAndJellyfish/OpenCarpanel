import type { HostDiagnostics } from "./desktop-api";

export type GameId = "f1-24" | "f1-25" | "ets2" | "ats" | "waiting";

export interface GameProfile {
  id: GameId;
  shortLabel: string;
  label: string;
  accent: string;
  family: "formula" | "truck" | "neutral";
}

const PROFILES: Record<GameId, GameProfile> = {
  "f1-24": {
    id: "f1-24",
    shortLabel: "F1 24",
    label: "EA Sports F1 24",
    accent: "#d9ff43",
    family: "formula",
  },
  "f1-25": {
    id: "f1-25",
    shortLabel: "F1 25",
    label: "F1 25 · 2026 Season Pack",
    accent: "#40e6d2",
    family: "formula",
  },
  ets2: {
    id: "ets2",
    shortLabel: "ETS2",
    label: "Euro Truck Simulator 2",
    accent: "#ffb74a",
    family: "truck",
  },
  ats: {
    id: "ats",
    shortLabel: "ATS",
    label: "American Truck Simulator",
    accent: "#ff6b62",
    family: "truck",
  },
  waiting: {
    id: "waiting",
    shortLabel: "AUTO",
    label: "等待游戏遥测",
    accent: "#aab4af",
    family: "neutral",
  },
};

export function gameProfile(diagnostics: HostDiagnostics): GameProfile {
  const candidate =
    diagnostics.activeAdapter ??
    (diagnostics.adapterSelection === "auto" ? "waiting" : diagnostics.adapterSelection);
  return candidate in PROFILES ? PROFILES[candidate as GameId] : PROFILES.waiting;
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
