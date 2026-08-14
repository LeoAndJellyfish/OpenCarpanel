import type { ScsPluginStatus } from "./desktop-api";

export type ScsGame = "ets2" | "ats";
export type ScsDiscoveryPhase =
  | "idle"
  | "searching"
  | "automatic"
  | "manual"
  | "not-found"
  | "failed";

export interface ScsDirectoryPresentation {
  directory: string;
  directoryState: string;
  chooseLabel: string;
}

export function isScsGame(game: string): game is ScsGame {
  return game === "ets2" || game === "ats";
}

export function scsDirectoryPresentation(
  status: ScsPluginStatus | null,
  phase: ScsDiscoveryPhase,
): ScsDirectoryPresentation {
  if (status) {
    return {
      directory: status.gameDirectory,
      directoryState: phase === "manual" ? "手动选择" : "Steam 已找到",
      chooseLabel: "重新选择",
    };
  }
  switch (phase) {
    case "searching":
      return {
        directory: "正在查找 Steam 游戏库…",
        directoryState: "查找中",
        chooseLabel: "选择文件夹",
      };
    case "not-found":
      return {
        directory: "未在 Steam 中找到",
        directoryState: "需手动选择",
        chooseLabel: "选择文件夹",
      };
    case "failed":
      return {
        directory: "自动查找不可用",
        directoryState: "需手动选择",
        chooseLabel: "选择文件夹",
      };
    default:
      return {
        directory: "等待自动查找",
        directoryState: "等待",
        chooseLabel: "选择文件夹",
      };
  }
}
