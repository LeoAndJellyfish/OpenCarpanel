import { describe, expect, it } from "vitest";

import {
  IDLE_UPDATE_PROGRESS,
  applyUpdateProgress,
  beginUpdateProgress,
  failUpdateProgress,
  formatBytes,
  formatUpdateBytes,
  updateProgressRatio,
} from "./update-progress";

describe("desktop update progress", () => {
  it("keeps downloaded bytes monotonic when channel messages are delayed", () => {
    const current = {
      phase: "downloading" as const,
      downloadedBytes: 80,
      totalBytes: 100,
    };
    expect(
      applyUpdateProgress(current, {
        phase: "downloading",
        downloadedBytes: 60,
        totalBytes: 100,
      }),
    ).toEqual(current);
  });

  it("does not invent a percentage without a total byte count", () => {
    const state = applyUpdateProgress(IDLE_UPDATE_PROGRESS, {
      phase: "downloading",
      downloadedBytes: 4_096,
      totalBytes: null,
    });
    expect(updateProgressRatio(state)).toBeNull();
    expect(formatUpdateBytes(state)).toBe("4.00 KB");
  });

  it("clamps determinate progress and marks installation complete", () => {
    const downloaded = applyUpdateProgress(IDLE_UPDATE_PROGRESS, {
      phase: "downloading",
      downloadedBytes: 120,
      totalBytes: 100,
    });
    expect(updateProgressRatio(downloaded)).toBe(1);
    expect(updateProgressRatio({ ...downloaded, phase: "installing" })).toBe(1);
  });

  it("resets a new operation and preserves evidence when it fails", () => {
    const checking = beginUpdateProgress("checking");
    expect(checking).toEqual({
      phase: "checking",
      downloadedBytes: 0,
      totalBytes: null,
    });
    const failed = failUpdateProgress(checking);
    expect(failed).toEqual({ ...checking, phase: "failed" });
    expect(applyUpdateProgress(failed, { phase: "verifying" })).toBe(failed);
  });

  it("formats byte counts at compact diagnostic precision", () => {
    expect(formatBytes(900)).toBe("900 B");
    expect(formatBytes(1_536)).toBe("1.50 KB");
    expect(formatBytes(12 * 1_024 * 1_024)).toBe("12.0 MB");
  });
});
