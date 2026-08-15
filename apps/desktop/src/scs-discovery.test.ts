import { describe, expect, it } from "vitest";

import type { ScsPluginStatus } from "./desktop-api";
import {
  isScsGame,
  scsBridgeNotice,
  scsDirectoryPresentation,
  scsRuntimeGame,
} from "./scs-discovery";

const status: ScsPluginStatus = {
  game: "ets2",
  gameDirectory: "D:\\SteamLibrary\\steamapps\\common\\Euro Truck Simulator 2",
  pluginPath: "D:\\SteamLibrary\\steamapps\\common\\Euro Truck Simulator 2\\bin\\win_x64\\plugins\\opensimdash-scs-telemetry.dll",
  state: "missing",
  bundledSha256: "bundle",
  installedSha256: null,
};

describe("SCS Steam discovery presentation", () => {
  it("distinguishes an automatic result from a manual selection", () => {
    expect(scsDirectoryPresentation(status, "automatic")).toMatchObject({
      directory: status.gameDirectory,
      directoryState: "Steam 已找到",
      chooseLabel: "重新选择",
    });
    expect(scsDirectoryPresentation(status, "manual").directoryState).toBe("手动选择");
  });

  it("keeps manual selection available while searching or after no match", () => {
    expect(scsDirectoryPresentation(null, "searching")).toMatchObject({
      directoryState: "查找中",
      chooseLabel: "选择文件夹",
    });
    expect(scsDirectoryPresentation(null, "not-found")).toMatchObject({
      directory: "未在 Steam 中找到",
      directoryState: "需手动选择",
    });
  });

  it("only classifies the two SCS games", () => {
    expect(isScsGame("ets2")).toBe(true);
    expect(isScsGame("ats")).toBe(true);
    expect(isScsGame("f1-25")).toBe(false);
  });

  it("checks the active SCS source before a fixed selection", () => {
    expect(scsRuntimeGame("ets2", "auto")).toBe("ets2");
    expect(scsRuntimeGame("ats", "ets2")).toBe("ats");
    expect(scsRuntimeGame("f1-25", "ets2")).toBe("ets2");
    expect(scsRuntimeGame("f1-25", "auto")).toBeNull();
  });

  it("only asks for action when the bridge is missing or outdated", () => {
    expect(scsBridgeNotice(status)).toMatchObject({
      message: "ETS2 SCS Bridge 尚未安装，任务数据暂不可用",
      actionLabel: "安装 Bridge",
    });
    expect(scsBridgeNotice({ ...status, state: "outdated" })).toMatchObject({
      message: "ETS2 SCS Bridge 需要更新，任务数据暂不可用",
      actionLabel: "更新 Bridge",
    });
    expect(scsBridgeNotice({ ...status, state: "current" })).toBeNull();
    expect(scsBridgeNotice(null)).toBeNull();
  });
});
