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
                ...(truck ? {} : { drs: wave > 0.72 ? "active" : "available" as const }),
              },
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

function readDemoGameId(value: string | null): BuiltinGameId {
  return BUILTIN_GAME_IDS.includes(value as BuiltinGameId)
    ? (value as BuiltinGameId)
    : "f1-24";
}
