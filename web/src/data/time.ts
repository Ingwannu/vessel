// Paseo's compact age (`packages/app/src/utils/time.ts` `describeAge`
// 36-58, `describeCompactTimeAgo` 80-84, `formatCompactTimeAgo` 86-88).
// The label alone: "now", "5m", "2h", "3d", "Jan 15". Never "ago", and
// never a seconds count. Under a minute is "now".

const MINUTE_MS = 60_000;
const HOUR_MS = 60 * MINUTE_MS;
const DAY_MS = 24 * HOUR_MS;
/** Past a week the elapsed count stops meaning anything. */
const ABSOLUTE_AFTER_MS = 7 * DAY_MS;

export type TimeResolution = "minute" | "hour" | "day" | "static";

export interface CompactTime {
  label: string;
  resolution: TimeResolution;
}

/** `date` as Paseo's compact age, read at `now`. */
export function compactTimeAgo(
  date: Date,
  now: Date = new Date(),
): CompactTime {
  const elapsedMs = now.getTime() - date.getTime();
  if (elapsedMs < MINUTE_MS) {
    return { label: "now", resolution: "minute" };
  }
  if (elapsedMs < HOUR_MS) {
    return {
      label: `${Math.floor(elapsedMs / MINUTE_MS)}m`,
      resolution: "minute",
    };
  }
  if (elapsedMs < DAY_MS) {
    return {
      label: `${Math.floor(elapsedMs / HOUR_MS)}h`,
      resolution: "hour",
    };
  }
  if (elapsedMs < ABSOLUTE_AFTER_MS) {
    return {
      label: `${Math.floor(elapsedMs / DAY_MS)}d`,
      resolution: "day",
    };
  }
  const month = date.toLocaleDateString("en-US", { month: "short" });
  return { label: `${month} ${String(date.getDate())}`, resolution: "static" };
}

/** How long until a compact label can change, so a row ticks then. */
export function compactDelay(resolution: TimeResolution): number | null {
  switch (resolution) {
    case "minute":
      return MINUTE_MS;
    case "hour":
      return HOUR_MS;
    case "day":
      return DAY_MS;
    case "static":
      return null;
  }
}
