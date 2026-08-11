import { lazy, Suspense } from "preact/compat";

import { DriveRoute } from "./routes/drive";

const EditRoute = lazy(async () => {
  const module = await import("./routes/edit");
  return { default: module.EditRoute };
});

export function App() {
  const isEditor = window.location.pathname.replace(/\/+$/, "") === "/edit";
  if (!isEditor) {
    return <DriveRoute />;
  }
  return (
    <Suspense
      fallback={
        <main class="connection-shell">
          <section class="connection-card" aria-live="polite">
            <p class="eyebrow">OpenCarpanel</p>
            <h1>正在载入布局编辑器</h1>
          </section>
        </main>
      }
    >
      <EditRoute />
    </Suspense>
  );
}
