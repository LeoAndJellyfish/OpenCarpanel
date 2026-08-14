import type { FuelState } from "../telemetry/store";

const WEATHER_LABELS: Readonly<Record<string, string>> = {
  clear: "CLEAR",
  light_cloud: "LIGHT CLOUD",
  overcast: "OVERCAST",
  light_rain: "LIGHT RAIN",
  heavy_rain: "HEAVY RAIN",
  storm: "STORM",
};

export function formatLapTime(milliseconds: number | null | undefined): string {
  if (!isFiniteNumber(milliseconds) || milliseconds < 0) {
    return "—";
  }
  const total = Math.round(milliseconds);
  const minutes = Math.floor(total / 60_000);
  const seconds = Math.floor((total % 60_000) / 1_000);
  const millis = total % 1_000;
  return `${minutes}:${seconds.toString().padStart(2, "0")}.${millis
    .toString()
    .padStart(3, "0")}`;
}

export function formatDelta(milliseconds: number | null | undefined): string {
  if (!isFiniteNumber(milliseconds)) {
    return "—";
  }
  const sign = milliseconds > 0 ? "+" : milliseconds < 0 ? "−" : "±";
  return `${sign}${(Math.abs(milliseconds) / 1_000).toFixed(3)}`;
}

export function formatDistance(metres: number | null | undefined): string {
  if (!isFiniteNumber(metres) || metres < 0) {
    return "—";
  }
  if (metres < 1_000) {
    return `${Math.round(metres)} M`;
  }
  const kilometres = metres / 1_000;
  return `${kilometres < 100 ? kilometres.toFixed(1) : Math.round(kilometres)} KM`;
}

export function formatDuration(seconds: number | null | undefined): string {
  if (!isFiniteNumber(seconds) || seconds < 0) {
    return "—";
  }
  const totalMinutes = Math.round(seconds / 60);
  const hours = Math.floor(totalMinutes / 60);
  const minutes = totalMinutes % 60;
  return hours > 0 ? `${hours} H ${minutes.toString().padStart(2, "0")} M` : `${minutes} M`;
}

export function formatSessionTime(milliseconds: number | null | undefined): string {
  return isFiniteNumber(milliseconds) ? formatDuration(milliseconds / 1_000) : "—";
}

export function formatTemperature(celsius: number | null | undefined): string {
  return isFiniteNumber(celsius) ? `${Math.round(celsius)}°` : "—";
}

export function formatPressure(pascals: number | null | undefined): string {
  return isFiniteNumber(pascals) && pascals >= 0 ? `${(pascals / 100_000).toFixed(2)}` : "—";
}

export function formatPercent(normalized: number | null | undefined): string {
  return isFiniteNumber(normalized)
    ? `${Math.round(Math.min(1, Math.max(0, normalized)) * 100)}%`
    : "—";
}

export function formatFuel(fuel: FuelState | undefined): string {
  if (isFiniteNumber(fuel?.remainingLaps)) {
    return `${fuel.remainingLaps.toFixed(1)} LAPS`;
  }
  if (isFiniteNumber(fuel?.kg)) {
    return `${fuel.kg.toFixed(1)} KG`;
  }
  if (isFiniteNumber(fuel?.liters)) {
    return `${fuel.liters.toFixed(0)} L`;
  }
  if (isFiniteNumber(fuel?.rangeKm)) {
    return `${Math.round(fuel.rangeKm)} KM`;
  }
  return "—";
}

export function fuelRatio(fuel: FuelState | undefined): number | undefined {
  const value = fuel?.liters ?? fuel?.kg;
  const capacity = fuel?.capacityLiters ?? fuel?.capacityKg;
  if (!isFiniteNumber(value) || !isFiniteNumber(capacity) || capacity <= 0) {
    return undefined;
  }
  return Math.min(1, Math.max(0, value / capacity));
}

export function weatherLabel(weather: string | null | undefined): string {
  return weather ? (WEATHER_LABELS[weather] ?? weather.replaceAll("_", " ").toUpperCase()) : "—";
}

export function textOrDash(value: string | null | undefined): string {
  const trimmed = value?.trim();
  return trimmed ? trimmed.toUpperCase() : "—";
}

function isFiniteNumber(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value);
}
