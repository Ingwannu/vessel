// Which windows the sidebar summary shows (Paseo usage/preferences.ts
// `effectiveUsagePins`, `toggleUsagePin`; the native view's pins): the
// person's list when they made one, else each source's default windows.
// A pin names the node's source and window ids, never a label (ADR 0107).
import type { UsagePin } from "../../wire/bridge";
import type { UsageReports, UsageWindow } from "../../wire/proto";
import { quotaPercent, type QuotaDisplay } from "./values";

function samePin(a: UsagePin, b: UsagePin): boolean {
  return a.source === b.source && a.window === b.window;
}

/** The windows of current reports from enabled sources, by source (the
 * native `summary_candidates`): a failed or never-read report has none. */
export function summaryWindows(
  reports: UsageReports,
): { source: UsagePin["source"]; windows: readonly UsageWindow[] }[] {
  if (reports.policy === "off") return [];
  return reports.reports.flatMap((entry) => {
    const source = reports.sources.find((item) => item.id === entry.source);
    if (source !== undefined && source.availability !== "enabled") return [];
    const state = entry.state;
    const observed =
      state.state === "available"
        ? state.observed
        : state.state === "refreshing" && state.origin === "available"
          ? state.retained
          : null;
    return observed === null
      ? []
      : [{ source: entry.source, windows: observed.report.windows }];
  });
}

/** The person's pins, or each source's defaults: its windows marked for
 * the summary, else its first window with a percent. */
export function effectivePins(
  pins: readonly UsagePin[] | null,
  reports: UsageReports,
  display: QuotaDisplay,
): UsagePin[] {
  if (pins !== null) return [...pins];
  const chosen: UsagePin[] = [];
  for (const { source, windows } of summaryWindows(reports)) {
    const withPercent = windows.filter(
      (window) =>
        quotaPercent(
          { used: window.used, remaining: window.remaining },
          display,
        ) !== null,
    );
    const marked = withPercent.filter(
      (window) => window.summary === "included",
    );
    for (const window of marked.length > 0 ? marked : withPercent.slice(0, 1)) {
      const pin = { source, window: window.id };
      if (!chosen.some((existing) => samePin(existing, pin))) chosen.push(pin);
    }
  }
  return chosen;
}

export function isPinned(
  pins: readonly UsagePin[] | null,
  pin: UsagePin,
  reports: UsageReports,
  display: QuotaDisplay,
): boolean {
  return effectivePins(pins, reports, display).some((pinned) =>
    samePin(pinned, pin),
  );
}

/** The first edit keeps the defaults that were shown, then toggles one. */
export function togglePin(
  pins: readonly UsagePin[] | null,
  pin: UsagePin,
  reports: UsageReports,
  display: QuotaDisplay,
): UsagePin[] {
  const selected = effectivePins(pins, reports, display);
  return selected.some((existing) => samePin(existing, pin))
    ? selected.filter((existing) => !samePin(existing, pin))
    : [...selected, pin];
}
