export type UpdateProgressEvent =
  | { phase: "preparing" }
  | {
      phase: "downloading";
      downloadedBytes: number;
      totalBytes: number | null;
    }
  | { phase: "verifying" }
  | { phase: "installing" };

export type UpdateProgressPhase =
  | "idle"
  | "checking"
  | UpdateProgressEvent["phase"]
  | "failed";

export interface UpdateProgressState {
  phase: UpdateProgressPhase;
  downloadedBytes: number;
  totalBytes: number | null;
}

export const IDLE_UPDATE_PROGRESS: UpdateProgressState = {
  phase: "idle",
  downloadedBytes: 0,
  totalBytes: null,
};

export function beginUpdateProgress(
  phase: "checking" | "preparing",
): UpdateProgressState {
  return { phase, downloadedBytes: 0, totalBytes: null };
}

export function applyUpdateProgress(
  current: UpdateProgressState,
  event: UpdateProgressEvent,
): UpdateProgressState {
  if (current.phase === "failed") return current;
  if (event.phase === "preparing") return beginUpdateProgress("preparing");
  if (event.phase !== "downloading") return { ...current, phase: event.phase };

  const downloadedBytes = Math.max(
    0,
    current.downloadedBytes,
    finiteBytes(event.downloadedBytes),
  );
  const announcedTotal = normalizeTotal(event.totalBytes);

  return {
    phase: "downloading",
    downloadedBytes,
    totalBytes: announcedTotal ?? current.totalBytes,
  };
}

export function failUpdateProgress(
  current: UpdateProgressState,
): UpdateProgressState {
  return { ...current, phase: "failed" };
}

export function updateProgressRatio(state: UpdateProgressState): number | null {
  if (state.phase === "installing") return 1;
  if (state.totalBytes === null || state.totalBytes <= 0) return null;
  return Math.min(1, Math.max(0, state.downloadedBytes / state.totalBytes));
}

export function updateProgressLabel(phase: UpdateProgressPhase): string {
  switch (phase) {
    case "checking":
      return "正在检查更新";
    case "preparing":
      return "正在准备更新";
    case "downloading":
      return "正在下载更新";
    case "verifying":
      return "正在验证签名";
    case "installing":
      return "正在启动安装";
    case "failed":
      return "更新未完成";
    default:
      return "";
  }
}

export function formatUpdateBytes(state: UpdateProgressState): string | null {
  if (state.downloadedBytes <= 0) return null;
  const downloaded = formatBytes(state.downloadedBytes);
  return state.totalBytes === null
    ? downloaded
    : `${downloaded} / ${formatBytes(state.totalBytes)}`;
}

export function formatBytes(bytes: number): string {
  const normalized = finiteBytes(bytes);
  if (normalized < 1_024) return `${Math.round(normalized)} B`;

  const units = ["KB", "MB", "GB", "TB"] as const;
  let value = normalized / 1_024;
  let unit: (typeof units)[number] = units[0];
  for (let index = 1; index < units.length && value >= 1_024; index += 1) {
    value /= 1_024;
    unit = units[index] ?? unit;
  }
  const fractionDigits = value >= 100 ? 0 : value >= 10 ? 1 : 2;
  return `${value.toFixed(fractionDigits)} ${unit}`;
}

function finiteBytes(value: number): number {
  return Number.isFinite(value) ? Math.max(0, value) : 0;
}

function normalizeTotal(value: number | null): number | null {
  if (value === null || !Number.isFinite(value) || value <= 0) return null;
  return value;
}
