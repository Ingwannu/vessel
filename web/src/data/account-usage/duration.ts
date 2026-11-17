// Localize the view's already-floored duration, not an ISO string or raw
// vendor seconds. Standard Intl unit words keep the viewer's language;
// the eventual localized surface owns reset/risk/freshness sentence copy.
import type { QuotaClock } from "./clock";

type DurationParts = Extract<QuotaClock, { kind: "reset" }>["parts"];
type DurationUnit = DurationParts[0]["unit"];

function intlUnit(unit: DurationUnit): "day" | "hour" | "minute" {
  switch (unit) {
    case "days":
      return "day";
    case "hours":
      return "hour";
    case "minutes":
      return "minute";
  }
}

/** The reference's two whole units, with locale-specific numbers and unit words. */
export function quotaDurationText(
  parts: DurationParts,
  locale: string,
): string {
  return parts
    .flatMap((part) =>
      part === undefined
        ? []
        : [
            new Intl.NumberFormat(locale, {
              style: "unit",
              unit: intlUnit(part.unit),
              unitDisplay: "narrow",
              maximumFractionDigits: 0,
            }).format(part.count),
          ],
    )
    .join(" ");
}
