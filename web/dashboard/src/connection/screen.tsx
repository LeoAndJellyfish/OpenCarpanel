import type { ConnectionView } from "./client";

export interface ConnectionScreenProps {
  readonly view: ConnectionView;
}

export function ConnectionScreen({ view }: ConnectionScreenProps) {
  return (
    <main class="connection-shell" data-state={view.phase}>
      <section class="connection-card" aria-live="polite">
        <p class="eyebrow">OpenSimDash</p>
        <h1>{view.phase === "connected" ? "仪表盘已连接" : "连接驾驶主机"}</h1>
        <p>{view.detail}</p>
        <span class="status-dot" aria-hidden="true" />
      </section>
    </main>
  );
}
