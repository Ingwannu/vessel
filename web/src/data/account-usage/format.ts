// Localized quota observations, not timestamps or raw provider numbers.
// The ledger's full observation date is unsuitable for a reset's small
// clock hint. A supplied device formatter preserves its time zone and
// 12/24-hour choice while day names follow the viewer's language.
import type { QuotaClock } from "./clock";
import type { QuotaPercent } from "./values";

export function quotaPercentText(value: QuotaPercent, locale: string): string {
  return new Intl.NumberFormat(locale, {
    style: "percent",
    maximumFractionDigits: 0,
  }).format(value.percent / 100);
}

/** Sentence joining belongs to localized surface copy, not this date policy. */
export type QuotaResetClock =
  | { kind: "time"; time: string }
  | { kind: "weekday" | "date"; day: string; time: string };

/** Exact time is available only for a future reset, never a run-out forecast. */
export function quotaResetClock(
  clock: QuotaClock,
  locale: string,
  deviceClock: Intl.DateTimeFormat,
): QuotaResetClock | null {
  if (clock.kind !== "reset") return null;
  const at = new Date(clock.at.milliseconds);
  const time = deviceClock.format(at);
  const timeZone = deviceClock.resolvedOptions().timeZone;
  switch (clock.distance) {
    case "day":
      return { kind: "time", time };
    case "week":
      return {
        kind: "weekday",
        day: new Intl.DateTimeFormat(locale, {
          weekday: "short",
          timeZone,
        }).format(at),
        time,
      };
    case "later":
      return {
        kind: "date",
        day: new Intl.DateTimeFormat(locale, {
          month: "short",
          day: "numeric",
          timeZone,
        }).format(at),
        time,
      };
  }
}
