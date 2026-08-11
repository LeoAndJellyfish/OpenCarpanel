export function formatSpeed(speedMps: number | undefined): string {
  if (speedMps === undefined) {
    return "—";
  }
  const kilometresPerHour = Math.max(0, Math.round(speedMps * 3.6));
  return String(kilometresPerHour).padStart(3, "0");
}
