import { describe, expect, it } from "vitest";

import type { HostDiagnostics } from "./desktop-api";
import { formatAge, gameProfile, telemetryIsLive } from "./model";

function diagnostics(activeAdapter: string | null, age: number | null): HostDiagnostics {
  return {
    status: "ok",
    version: "0.2.0",
    protocolVersion: 1,
    adapter: activeAdapter ?? "auto",
    adapterSelection: "auto",
    activeAdapter,
    supportedAdapters: [],
    uptimeMs: 1,
    telemetry: {
      packetsReceived: activeAdapter ? 1 : 0,
      packetsRecognized: activeAdapter ? 1 : 0,
      packetErrors: 0,
      lastPacketAgeMs: age,
      snapshotsPublished: activeAdapter ? 1 : 0,
      eventResyncs: 0,
    },
    connections: { active: 0, limit: 8 },
  };
}

describe("desktop presentation model", () => {
  it("tracks the active game instead of guessing from telemetry fields", () => {
    expect(gameProfile(diagnostics("f1-25", 12)).accent).toBe("#40e6d2");
    expect(gameProfile(diagnostics("ets2", 12)).family).toBe("truck");
    expect(gameProfile(diagnostics(null, null)).id).toBe("waiting");
  });

  it("marks stale packets as not live", () => {
    expect(telemetryIsLive(diagnostics("f1-24", 80))).toBe(true);
    expect(telemetryIsLive(diagnostics("f1-24", 2_500))).toBe(false);
    expect(telemetryIsLive(diagnostics(null, null))).toBe(false);
  });

  it("formats packet age at useful diagnostic precision", () => {
    expect(formatAge(13)).toBe("13 ms");
    expect(formatAge(1_250)).toBe("1.3 s");
    expect(formatAge(null)).toBe("—");
  });
});
