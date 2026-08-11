import type { Gear } from "@opencarpanel/widget-sdk";

export function formatGear(gear: Gear | undefined): string {
  if (gear === "neutral") {
    return "N";
  }
  if (gear === "reverse") {
    return "R";
  }
  if (gear && typeof gear === "object" && "forward" in gear) {
    return String(gear.forward);
  }
  return "–";
}
