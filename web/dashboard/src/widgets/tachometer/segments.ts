export const REV_SEGMENT_COUNT = 20;

export function activeSegmentCount(progress: number | undefined): number {
  if (progress === undefined || !Number.isFinite(progress)) {
    return 0;
  }
  return Math.round(Math.min(1, Math.max(0, progress)) * REV_SEGMENT_COUNT);
}

export function activationRank(index: number): number {
  return index < REV_SEGMENT_COUNT / 2
    ? index * 2
    : (REV_SEGMENT_COUNT - 1 - index) * 2 + 1;
}
