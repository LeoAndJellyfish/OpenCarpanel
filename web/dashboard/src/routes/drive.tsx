import { cloneLayout, type LayoutDocument } from "@opensimdash/widget-sdk";
import { useEffect, useMemo, useState } from "preact/hooks";

import { loadLayout } from "../api/layouts";
import { ConnectionScreen } from "../connection/screen";
import { Dashboard } from "../dashboard/dashboard";
import { useDashboardBreakpoint } from "../dashboard/breakpoint";
import { gamePresentation } from "../dashboard/game-profile";
import { useTelemetryRuntime } from "../telemetry/use-runtime";

export function DriveRoute() {
  const runtime = useTelemetryRuntime();
  const breakpoint = useDashboardBreakpoint();
  const presentation = useMemo(
    () => gamePresentation(runtime.gameId, runtime.plugins),
    [runtime.gameId, runtime.plugins],
  );
  const [layout, setLayout] = useState<LayoutDocument>(() =>
    cloneLayout(presentation.defaultLayout),
  );

  useEffect(() => {
    setLayout(cloneLayout(presentation.defaultLayout));
    if (
      runtime.connection.phase !== "connected" ||
      (import.meta.env.DEV && isDemoMode())
    ) {
      return;
    }
    let active = true;
    void loadLayout(presentation.layoutId)
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
  }, [presentation.defaultLayout, presentation.layoutId, runtime.connection.phase]);

  if (!runtime.hasConnected) {
    return <ConnectionScreen view={runtime.connection} />;
  }
  return (
    <Dashboard
      loop={runtime.loop}
      connection={runtime.connection}
      layout={layout}
      breakpoint={breakpoint}
      presentation={presentation}
    />
  );
}

function isDemoMode(): boolean {
  return new URLSearchParams(window.location.search).has("demo");
}
