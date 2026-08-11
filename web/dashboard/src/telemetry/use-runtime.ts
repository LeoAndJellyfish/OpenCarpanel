import type { ServerMessage } from "@opencarpanel/widget-sdk";
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
  readonly hasConnected: boolean;
  readonly loop: TelemetryRenderLoop;
}

export function useTelemetryRuntime(): TelemetryRuntime {
  const [connection, setConnection] = useState<ConnectionView>(initialView);
  const [hasConnected, setHasConnected] = useState(false);
  const [store] = useState(
    () =>
      new TelemetryStore({
        reducedMotion: window.matchMedia("(prefers-reduced-motion: reduce)").matches,
      }),
  );
  const [loop] = useState(() => new TelemetryRenderLoop(store));
  const demoMode =
    import.meta.env.DEV && new URLSearchParams(window.location.search).has("demo");

  useEffect(() => {
    if (demoMode) {
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
                gameId: "f1-24",
                schemaVersion: 1,
                sequence,
                sessionId: "dashboard-visual-demo",
              },
              vehicle: {
                speedMps: 76 + Math.sin(elapsedMs / 1_300) * 8,
                rpm: 8_900 + wave * 2_800,
                rpmMax: 12_000,
                revLights: 0.58 + wave * 0.4,
                throttle: 0.78 + wave * 0.2,
                brake: 0,
                gear: { forward: 7 },
                drs: wave > 0.72 ? "active" : "available",
              },
            },
          },
          elapsedMs,
        );
      };
      setConnection({ phase: "connected", detail: "本地视觉演示数据正在运行。" });
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
  }, [demoMode, loop, store]);

  return { connection, hasConnected, loop };
}
