// The words of a usage card's time lines (Paseo usage/reset-line.tsx and
// card.tsx freshness): pure, so the sidebar summary and modal reuse them.
import type { QuotaClock } from "../../data/account-usage/clock";
import { quotaAge, type QuotaInstant } from "../../data/account-usage/clock";
import { quotaDurationText } from "../../data/account-usage/duration";
import { quotaResetClock } from "../../data/account-usage/format";

/** Paseo reset-line.tsx: the countdown and its exact time, or the forecast. */
export function resetText(
  clock: QuotaClock,
  locale: string,
  deviceClock: Intl.DateTimeFormat,
): { text: string; atRisk: boolean } | null {
  switch (clock.kind) {
    case "unreported":
      return null;
    // Paseo format.ts formatResetLabel: "resetting now" for a current read,
    // "Reset time passed" for values kept from before the reset.
    case "resetting":
      return { text: "Resetting now", atRisk: false };
    case "reset_passed":
      return { text: "Reset time passed", atRisk: false };
    case "reset": {
      const duration = quotaDurationText(clock.parts, locale);
      const at = quotaResetClock(clock, locale, deviceClock);
      const time =
        at === null
          ? null
          : at.kind === "time"
            ? at.time
            : `${at.day} ${at.time}`;
      return {
        text:
          time === null
            ? `Resets in ${duration}`
            : `Resets in ${duration} · ${time}`,
        atRisk: false,
      };
    }
    case "risk_now":
      return {
        text: `Source forecast: runs out now · Source forecast shortfall: ${clock.shortfall.format(locale)}`,
        atRisk: true,
      };
    case "risk":
      return {
        text: `Source forecast: runs out in ${quotaDurationText(clock.parts, locale)} · Source forecast shortfall: ${clock.shortfall.format(locale)}`,
        atRisk: true,
      };
  }
}

/** "Updated 10m ago": the shown success, never a failed attempt. */
export function freshnessText(
  at: QuotaInstant | null,
  now: QuotaInstant,
): string {
  const age = quotaAge(at, now);
  if (age === null) return "No successful update";
  if (age.kind === "now") return "Updated just now";
  const unit = age.unit === "days" ? "d" : age.unit === "hours" ? "h" : "m";
  return `Updated ${String(age.count)}${unit} ago`;
}
