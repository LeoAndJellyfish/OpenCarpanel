import { useEffect, useState } from "preact/hooks";

import { TelemetryConnection, type ConnectionView } from "./connection/client";
import { consumePairingToken } from "./connection/pairing";

const initialView: ConnectionView = {
  phase: "connecting",
  detail: "正在连接本地 Host…",
};

export function App() {
  const [view, setView] = useState<ConnectionView>(initialView);

  useEffect(() => {
    const pairingToken = consumePairingToken(window.location, window.history);
    const connection = new TelemetryConnection(setView);
    connection.start(pairingToken);
    return () => connection.stop();
  }, []);

  return (
    <main class="connection-shell" data-state={view.phase}>
      <section class="connection-card" aria-live="polite">
        <p class="eyebrow">OpenCarpanel</p>
        <h1>{view.phase === "connected" ? "仪表盘已连接" : "连接驾驶主机"}</h1>
        <p>{view.detail}</p>
        <span class="status-dot" aria-hidden="true" />
      </section>
    </main>
  );
}
