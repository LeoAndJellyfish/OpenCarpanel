import { describe, expect, it } from "vitest";
import { BUILTIN_GAME_PLUGINS, type GamePluginMetadata } from "@opencarpanel/widget-sdk";

import type { HostDiagnostics } from "./desktop-api";
import { formatAge, gameProfile, telemetryIsLive } from "./model";

function diagnostics(activeAdapter: string | null, age: number | null): HostDiagnostics {
  return {
    status: "ok",
    version: "0.4.0",
    protocolVersion: 1,
    adapter: activeAdapter ?? "auto",
    adapterSelection: "auto",
    activeAdapter,
    supportedAdapters: BUILTIN_GAME_PLUGINS.map((plugin) => ({
      plugin: structuredClone(plugin),
      id: plugin.id,
      displayName: plugin.name,
      protocolVersion: plugin.protocolVersion,
      capabilities: [...plugin.capabilities],
      packetsRecognized: 0,
      lastPacketAgeMs: null,
    })),
    pluginLoadIssues: [],
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
    expect(gameProfile(diagnostics("f1-25", 12)).accent).toBe(
      BUILTIN_GAME_PLUGINS.find((plugin) => plugin.id === "f1-25")?.presentation.theme.accent,
    );
    expect(gameProfile(diagnostics("ets2", 12)).family).toBe("truck");
    expect(gameProfile(diagnostics(null, null)).id).toBe("waiting");
  });

  it("builds presentation for an installed plugin without a hardcoded id", () => {
    const value = diagnostics("community-sim", 12);
    const plugin: GamePluginMetadata = structuredClone(BUILTIN_GAME_PLUGINS[0]);
    plugin.id = "community-sim";
    plugin.name = "Community Sim";
    plugin.source = "installed";
    plugin.presentation.shortName = "CSIM";
    plugin.presentation.family = "generic";
    plugin.presentation.theme.accent = "#123456";
    value.supportedAdapters.push({
      plugin,
      id: plugin.id,
      displayName: plugin.name,
      protocolVersion: plugin.protocolVersion,
      capabilities: [...plugin.capabilities],
      packetsRecognized: 1,
      lastPacketAgeMs: 12,
    });

    expect(gameProfile(value)).toEqual({
      id: "community-sim",
      shortLabel: "CSIM",
      label: "Community Sim",
      accent: "#123456",
      family: "neutral",
    });
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
