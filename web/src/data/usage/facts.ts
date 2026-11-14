// Reported subtotals keep their coverage and units. No prices, money
// arithmetic or token sums happen here. Browser numbers cannot exactly
// represent every wire u64: say so rather than print rounded fake digits.
import type { OptionalUsageReports, UsageReport } from "../../wire/proto";

export interface UsageFact {
  label: string;
  value: string;
  detail: string | null;
}

const OPTIONAL: readonly {
  key: keyof OptionalUsageReports;
  label: string;
}[] = [
  { key: "thought", label: "Reasoning tokens" },
  { key: "cached_read", label: "Cache read tokens" },
  { key: "cached_write", label: "Cache write tokens" },
];

/** Compact quantities use the viewer's locale; missing and reported zero differ. */
export function usageCount(
  value: number | null | undefined,
  locale: string,
): string {
  if (value === null || value === undefined) return "Not reported";
  if (!Number.isSafeInteger(value) || value < 0) {
    return "Not exactly representable in this browser";
  }
  return new Intl.NumberFormat(locale, {
    notation: "compact",
    maximumFractionDigits: 1,
  }).format(value);
}

/** Local readable timestamps; invalid input is never shown as an ISO literal. */
export function usageTime(value: string, locale: string): string {
  const time = Date.parse(value);
  return Number.isFinite(time)
    ? new Intl.DateTimeFormat(locale, {
        dateStyle: "medium",
        timeStyle: "short",
      }).format(time)
    : "Not available";
}

/** Full node-provided token/cost facts, including incomplete optional reports. */
export function usageFacts(
  report: UsageReport,
  locale: string,
): readonly UsageFact[] {
  const totals = report.totals;
  const number = new Intl.NumberFormat(locale);
  const turns = totals.reported + totals.unreported;
  const coverage = (count: number) =>
    `${number.format(count)} of ${number.format(turns)} turns reported`;
  const facts: UsageFact[] = [
    {
      label: "Total tokens",
      value: usageCount(totals.tokens?.total, locale),
      detail: coverage(totals.reported),
    },
    {
      label: "Input tokens",
      value: usageCount(totals.tokens?.input, locale),
      detail: coverage(totals.reported),
    },
    {
      label: "Output tokens",
      value: usageCount(totals.tokens?.output, locale),
      detail: coverage(totals.reported),
    },
    ...OPTIONAL.map(({ key, label }) => ({
      label,
      value: usageCount(totals.tokens?.[key], locale),
      detail: coverage(totals.optional_reports[key]),
    })),
    {
      label: "Turns without token reports",
      value: number.format(totals.unreported),
      detail: null,
    },
  ];
  switch (report.cost_visibility) {
    case "withheld":
      facts.push({
        label: "Reported cost",
        value: "Hidden for this device",
        detail: "Operate access is needed to see cost.",
      });
      break;
    case "permitted":
      if (totals.costs.length === 0) {
        facts.push({
          label: "Reported cost",
          value: "Not reported",
          detail: null,
        });
      }
      for (const cost of totals.costs) {
        const amount = new Intl.NumberFormat(locale, {
          maximumSignificantDigits: 17,
        }).format(cost.amount);
        facts.push({
          label: `Reported cost (${cost.unit})`,
          value: amount,
          detail:
            "Only reported increments attributed to this interval; no price estimates.",
        });
      }
      break;
  }
  if (totals.unaccounted_costs > 0) {
    facts.push({
      label: "Unattributed cost reports",
      value: number.format(totals.unaccounted_costs),
      detail:
        "Unknown timing or numerical precision prevents assigning these reports to this interval.",
    });
  }
  if (report.incomplete || totals.token_accounting === "overflow") {
    facts.push({
      label: "Coverage",
      value: "Partial report",
      detail:
        totals.token_accounting === "overflow"
          ? "Some token totals exceed the supported counter range."
          : "Retention or grouping limits omitted data; totals cover retained samples.",
    });
  }
  return facts;
}
