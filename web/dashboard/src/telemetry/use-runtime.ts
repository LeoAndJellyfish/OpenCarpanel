import {
  BUILTIN_GAME_IDS,
  type BuiltinGameId,
  type ServerMessage,
} from "@opencarpanel/widget-sdk";
import { useEffect, useState } from "preact/hooks";

import { TelemetryConnection, type ConnectionView } from "../connection/client";
import { consumePairingToken } from "../connection/pairing";
import { TelemetryRenderLoop } from "./render-loop";
import { TelemetryStore } from "./store";

const initialView: ConnectionView = {
  phase: "connecting",
  detail: "正在连接本地 Host…",
};

export interface TelemetryRuntime {
  readonly connection: ConnectionView;
  readonly gameId: string | undefined;
  readonly hasConnected: boolean;
  readonly loop: TelemetryRenderLoop;
}

export function useTelemetryRuntime(): TelemetryRuntime {
  const [connection, setConnection] = useState<ConnectionView>(initialView);
  const [gameId, setGameId] = useState<string>();
  const [hasConnected, setHasConnected] = useState(false);
  const [store] = useState(
    () =>
      new TelemetryStore({
        reducedMotion: window.matchMedia("(prefers-reduced-motion: reduce)").matches,
      }),
  );
  const [loop] = useState(() => new TelemetryRenderLoop(store));
  const search = new URLSearchParams(window.location.search);
  const demoMode = import.meta.env.DEV && search.has("demo");
  const demoGameId = readDemoGameId(search.get("game"));

  useEffect(() => {
    if (demoMode) {
      const truck = demoGameId === "ets2" || demoGameId === "ats";
      let sequence = 0;
      const publishDemoFrame = () => {
        const elapsedMs = performance.now();
        const wave = (Math.sin(elapsedMs / 920) + 1) / 2;
        sequence += 1;
        store.ingest(
          {
            capturedAtUs: Math.round(elapsedMs * 1_000),
            seq: sequence,
            data: {
              meta: {
                capturedAt: Math.round(elapsedMs * 1_000),
                gameId: demoGameId,
                schemaVersion: 1,
                sequence,
                sessionId: "dashboard-visual-demo",
              },
              vehicle: {
                speedMps: truck
                  ? 25 + Math.sin(elapsedMs / 1_900) * 4
                  : 76 + Math.sin(elapsedMs / 1_300) * 8,
                rpm: truck ? 1_250 + wave * 650 : 8_900 + wave * 2_800,
                rpmMax: truck ? 2_500 : 12_000,
                revLights: 0.58 + wave * 0.4,
                throttle: 0.78 + wave * 0.2,
                brake: 0,
                gear: { forward: truck ? 10 : 7 },
                ...(truck
                  ? {
                      fuelCapacityLiters: 800,
                      fuelLiters: 318 + wave * 8,
                      fuelRangeKm: 612,
                      fuelWarning: false,
                    }
                  : {
                      drs: wave > 0.72 ? "active" as const : "available" as const,
                      fuelCapacityKg: 110,
                      fuelKg: 37.4 - wave * 2,
                      fuelRemainingLaps: 8.4 - wave * 0.3,
                      fuelWarning: false,
                    }),
              },
              ...(truck
                ? {
                    navigation: {
                      distanceM: 286_400 - (elapsedMs % 120_000) * 0.02,
                      speedLimitMps: 22.22,
                      timeS: 15_120,
                    },
                    job: {
                      active: true,
                      cargo: "Electronics",
                      cargoLoaded: true,
                      cargoMassKg: 18_200,
                      destinationCity: demoGameId === "ats" ? "Denver" : "Prague",
                      plannedDistanceKm: 412,
                      sourceCity: demoGameId === "ats" ? "Salt Lake City" : "Berlin",
                      special: false,
                    },
                    lights: {
                      highBeam: false,
                      leftIndicator: wave > 0.82,
                      lowBeam: true,
                      rightIndicator: false,
                    },
                  }
                : {
                    lap: {
                      current: 12,
                      currentTimeMs: 64_100 + (elapsedMs % 27_000),
                      deltaToBestMs: Math.round((wave - 0.5) * 460),
                      invalid: false,
                      lastTimeMs: 91_422,
                      penaltiesSeconds: 0,
                      position: 4,
                      sector: 2,
                    },
                    session: {
                      raceFlag: "green" as const,
                      remainingTimeMs: 2_412_000,
                      safetyCarStatus: "none" as const,
                      sessionType: "race",
                      totalLaps: 57,
                      trackId: "bahrain",
                    },
                    conditions: {
                      airTemperatureC: 28,
                      trackTemperatureC: 37,
                      weather: "clear" as const,
                    },
                    tyres: {
                      ageLaps: 7,
                      visualCompound: 17,
                      frontLeft: tyreDemo(92 + wave * 3, 0.12),
                      frontRight: tyreDemo(94 + wave * 2, 0.14),
                      rearLeft: tyreDemo(88 + wave * 4, 0.17),
                      rearRight: tyreDemo(89 + wave * 3, 0.18),
                    },
                    damage: {
                      engine: 0.03,
                      floor: 0.01,
                      frontLeftWing: 0,
                      frontRightWing: 0,
                      gearbox: 0.02,
                    },
                    ...(demoGameId === "f1-25"
                      ? {
                          aero: {
                            available: true,
                            mode: wave > 0.45 ? "straight" as const : "corner" as const,
                            overtakeActive: wave > 0.82,
                            overtakeAvailable: true,
                            regulations2026: true,
                          },
                        }
                      : {}),
                  }),
            },
          },
          elapsedMs,
        );
      };
      setConnection({ phase: "connected", detail: "本地视觉演示数据正在运行。" });
      setGameId(demoGameId);
      setHasConnected(true);
      publishDemoFrame();
      const interval = window.setInterval(publishDemoFrame, 1_000 / 60);
      const detachVisibility = loop.attachVisibility(document, publishDemoFrame);
      return () => {
        window.clearInterval(interval);
        detachVisibility();
        loop.destroy();
      };
    }

    const pairingToken = consumePairingToken(window.location, window.history);
    let staleTimer: number | undefined;
    const observeView = (nextView: ConnectionView) => {
      setConnection(nextView);
      if (nextView.phase === "connected") {
        setHasConnected(true);
      } else {
        store.setStale(true);
      }
    };
    const observeMessage = (message: ServerMessage) => {
      if (message.type === "snapshot") {
        store.ingest(message, performance.now());
        setGameId(
          typeof message.data.meta?.gameId === "string"
            ? message.data.meta.gameId
            : undefined,
        );
        if (staleTimer !== undefined) {
          window.clearTimeout(staleTimer);
        }
        staleTimer = window.setTimeout(() => store.setStale(true), 750);
      } else if (message.type === "stale") {
        store.setStale(true);
      }
    };
    const telemetryConnection = new TelemetryConnection(observeView, observeMessage);
    const detachVisibility = loop.attachVisibility(document, () =>
      telemetryConnection.requestSnapshot(),
    );
    const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)");
    const handleMotionPreference = () => {
      store.setReducedMotion(reducedMotion.matches, performance.now());
    };
    reducedMotion.addEventListener("change", handleMotionPreference);
    telemetryConnection.start(pairingToken);
    return () => {
      if (staleTimer !== undefined) {
        window.clearTimeout(staleTimer);
      }
      reducedMotion.removeEventListener("change", handleMotionPreference);
      detachVisibility();
      telemetryConnection.stop();
      loop.destroy();
    };
  }, [demoGameId, demoMode, loop, store]);

  return { connection, gameId, hasConnected, loop };
}

function tyreDemo(temperatureC: number, wear: number) {
  return {
    damage: 0,
    innerTemperatureC: temperatureC,
    pressurePa: 145_000,
    surfaceTemperatureC: temperatureC - 2,
    wear,
  };
}

function readDemoGameId(value: string | null): BuiltinGameId {
  return BUILTIN_GAME_IDS.includes(value as BuiltinGameId)
    ? (value as BuiltinGameId)
    : "f1-24";
}
