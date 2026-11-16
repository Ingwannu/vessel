// Pick a summary representation that fits, following Paseo pinned.ts.
// The minimum track width avoids noisy slivers; it is never a minimum
// fill. A zero percentage stays zero even when its track is visible.
export type QuotaPinnedLayout = "meters" | "labels" | "percents";
export const MIN_QUOTA_METER_WIDTH = 16;
export const QUOTA_METER_GAP = 4;

export interface QuotaLayoutMeasurements {
  readonly availableWidth: number | null;
  readonly labelsWidth: number | null;
  readonly percentsWidth: number | null;
  readonly windowCount: number;
}

/** Before measurement, labels avoid flashing meters that turn out not to fit. */
export function quotaPinnedLayout(
  input: QuotaLayoutMeasurements,
): QuotaPinnedLayout {
  const { availableWidth, labelsWidth, percentsWidth, windowCount } = input;
  if (availableWidth === null || labelsWidth === null || percentsWidth === null)
    return "labels";
  if (
    ![availableWidth, labelsWidth, percentsWidth].every(
      (width) => Number.isFinite(width) && width >= 0,
    ) ||
    !Number.isSafeInteger(windowCount) ||
    windowCount < 0
  )
    return "labels";
  const needed =
    labelsWidth + windowCount * (MIN_QUOTA_METER_WIDTH + QUOTA_METER_GAP);
  if (!Number.isFinite(needed)) return "labels";
  if (needed <= availableWidth) return "meters";
  if (labelsWidth <= availableWidth) return "labels";
  return percentsWidth < labelsWidth ? "percents" : "labels";
}
