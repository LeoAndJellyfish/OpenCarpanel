import type { BreakpointName } from "@opensimdash/widget-sdk";
import { useEffect, useState } from "preact/hooks";

export function selectBreakpoint(width: number, height: number): BreakpointName {
  if (width >= 1_280 && height >= 720) {
    return "desktop";
  }
  if (width >= 700 && height >= 600) {
    return "tablet";
  }
  return height > width ? "phonePortrait" : "phoneLandscape";
}

export function useDashboardBreakpoint(): BreakpointName {
  const [breakpoint, setBreakpoint] = useState(() =>
    selectBreakpoint(window.innerWidth, window.innerHeight),
  );
  useEffect(() => {
    const update = () => setBreakpoint(selectBreakpoint(window.innerWidth, window.innerHeight));
    window.addEventListener("resize", update, { passive: true });
    return () => window.removeEventListener("resize", update);
  }, []);
  return breakpoint;
}
