import { describe, expect, it } from "vitest";

import type { ScsPluginStatus } from "./desktop-api";
import { isScsGame, scsDirectoryPresentation } from "./scs-discovery";

const status: ScsPluginStatus = {
  game: "ets2",
  gameDirectory: "D:\\SteamLibrary\\steamapps\\common\\Euro Truck Simulator 2",
  pluginPath: "D:\\SteamLibrary\\steamapps\\common\\Euro Truck Simulator 2\\bin\\win_x64\\plugins\\opencarpanel-scs-telemetry.dll",
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
});
