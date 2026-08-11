import { DEFAULT_LAYOUT, type LayoutDocument } from "@opencarpanel/widget-sdk";
import { useEffect, useState } from "preact/hooks";

import { loadLayout } from "../api/layouts";
import { ConnectionScreen } from "../connection/screen";
import { Dashboard } from "../dashboard/dashboard";
import { useDashboardBreakpoint } from "../dashboard/breakpoint";
import { useTelemetryRuntime } from "../telemetry/use-runtime";

export function DriveRoute() {
  const runtime = useTelemetryRuntime();
  const breakpoint = useDashboardBreakpoint();
  const [layout, setLayout] = useState<LayoutDocument>(DEFAULT_LAYOUT);

  useEffect(() => {
    if (runtime.connection.phase !== "connected" || import.meta.env.DEV && isDemoMode()) {
      return;
    }
    let active = true;
    void loadLayout("default")
      .then((loaded) => {
        if (active) {
          setLayout(loaded.document);
        }
      })
      .catch(() => {
        // Keep the validated built-in layout when persistence is temporarily unavailable.
      });
    return () => {
      active = false;
    };
  }, [runtime.connection.phase]);

  if (!runtime.hasConnected) {
    return <ConnectionScreen view={runtime.connection} />;
  }
  return (
    <Dashboard
      loop={runtime.loop}
      connection={runtime.connection}
      layout={layout}
      breakpoint={breakpoint}
    />
  );
}

function isDemoMode(): boolean {
  return new URLSearchParams(window.location.search).has("demo");
}
