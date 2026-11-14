// Reported subtotals keep their coverage and units. No prices, money
// arithmetic or token sums happen here. Browser numbers cannot exactly
// represent every wire u64: say so rather than print rounded fake digits.
import type {
  OptionalUsageReports,
  UsageReport,
  UsageTotals,
} from "../../wire/proto";

/** One existing Settings row; descriptions retain the limits of its value. */
export interface UsageFact {
  label: string;
  value: string;
  detail: string | null;
}

const TOKEN_FRACTION_DIGITS = 1;
const COST_SIGNIFICANT_DIGITS = 17;
const MIN_TOKEN_COUNT = 0;
const OPTIONAL: readonly { key: keyof OptionalUsageReports; label: string }[] =
  [
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
  if (!Number.isSafeInteger(value) || value < MIN_TOKEN_COUNT) {
    return "Not exactly representable in this browser";
  }
  return new Intl.NumberFormat(locale, {
    notation: "compact",
    maximumFractionDigits: TOKEN_FRACTION_DIGITS,
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

function tokenFacts(totals: UsageTotals, locale: string): UsageFact[] {
  const number = new Intl.NumberFormat(locale);
  const turns = totals.reported + totals.unreported;
  const coverage = (count: number) =>
    Number.isSafeInteger(count) && Number.isSafeInteger(turns)
      ? `${number.format(count)} of ${number.format(turns)} turns reported`
      : "Turn coverage is not exactly representable in this browser.";
  return [
    ...(["total", "input", "output"] as const).map((key) => ({
      label: `${key === "total" ? "Total" : key === "input" ? "Input" : "Output"} tokens`,
      value: usageCount(totals.tokens?.[key], locale),
      detail: coverage(totals.reported),
    })),
    ...OPTIONAL.map(({ key, label }) => ({
      label,
      value: usageCount(totals.tokens?.[key], locale),
      detail: coverage(totals.optional_reports[key]),
    })),
    {
      label: "Turns without token reports",
      value: usageCount(totals.unreported, locale),
      detail: null,
    },
  ];
}

function costFacts(report: UsageReport, locale: string): UsageFact[] {
  switch (report.cost_visibility) {
    case "withheld":
      return [
        {
          label: "Reported cost",
          value: "Hidden for this device",
          detail: "Operate access is needed to see cost.",
        },
      ];
    case "permitted":
      return report.totals.costs.length === 0
        ? [{ label: "Reported cost", value: "Not reported", detail: null }]
        : report.totals.costs.map((cost) => ({
            label: `Reported cost (${cost.unit})`,
            value: new Intl.NumberFormat(locale, {
              maximumSignificantDigits: COST_SIGNIFICANT_DIGITS,
            }).format(cost.amount),
            detail:
              "Only reported increments attributed to this interval; no price estimates.",
          }));
  }
}

function missingFacts(report: UsageReport, locale: string): UsageFact[] {
  const facts: UsageFact[] = [];
  if (report.totals.unaccounted_costs > 0) {
    facts.push({
      label: "Unattributed cost reports",
      value: usageCount(report.totals.unaccounted_costs, locale),
      detail:
        "Unknown timing or numerical precision prevents assigning these reports to this interval.",
    });
  }
  if (report.incomplete || report.totals.token_accounting === "overflow") {
    facts.push({
      label: "Coverage",
      value: "Partial report",
      detail:
        report.totals.token_accounting === "overflow"
          ? "Some token totals exceed the supported counter range."
          : "Retention or grouping limits omitted data; totals cover retained samples.",
    });
  }
  return facts;
}

/** Full node-provided facts: no optional coverage or cost gap disappears in formatting. */
export function usageFacts(
  report: UsageReport,
  locale: string,
): readonly UsageFact[] {
  return [
    ...tokenFacts(report.totals, locale),
    ...costFacts(report, locale),
    ...missingFacts(report, locale),
  ];
}

/** Scope/time provenance is distinct from the reported totals themselves. */
export function usageCoverage(
  report: UsageReport,
  locale: string,
): readonly UsageFact[] {
  return [
    { label: "Observed", value: usageTime(report.at, locale), detail: null },
    {
      label: "Accounting began",
      value: usageTime(report.accounting_started, locale),
      detail: "Earlier history is not backfilled.",
    },
    {
      label: "Retained since",
      value: usageTime(report.retained_from, locale),
      detail: "Only retained reported samples are counted.",
    },
  ];
}
