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

export interface ScsBridgeNotice {
  message: string;
  actionLabel: string;
}

export function isScsGame(game: string): game is ScsGame {
  return game === "ets2" || game === "ats";
}

export function scsRuntimeGame(
  activeAdapter: string | null | undefined,
  adapterSelection: string,
): ScsGame | null {
  if (activeAdapter && isScsGame(activeAdapter)) {
    return activeAdapter;
  }
  return isScsGame(adapterSelection) ? adapterSelection : null;
}

export function scsBridgeNotice(status: ScsPluginStatus | null): ScsBridgeNotice | null {
  if (!status || status.state === "current") {
    return null;
  }
  const game = status.game.toUpperCase();
  return status.state === "missing"
    ? {
        message: `${game} SCS Bridge 尚未安装，任务数据暂不可用`,
        actionLabel: "安装 Bridge",
      }
    : {
        message: `${game} SCS Bridge 需要更新，任务数据暂不可用`,
        actionLabel: "更新 Bridge",
      };
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
