export interface LinearInterpolation {
  readonly from: number;
  readonly to: number;
  readonly startedAtMs: number;
  readonly durationMs: number;
}

export function interpolateLinear(sample: LinearInterpolation, nowMs: number): number {
  if (sample.durationMs <= 0 || nowMs >= sample.startedAtMs + sample.durationMs) {
    return sample.to;
  }
  if (nowMs <= sample.startedAtMs) {
    return sample.from;
  }
  const progress = (nowMs - sample.startedAtMs) / sample.durationMs;
  return sample.from + (sample.to - sample.from) * progress;
}
