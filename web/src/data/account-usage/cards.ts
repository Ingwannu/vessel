// Paseo's usage cards (usage/card.tsx, window-bar.tsx, balance-bar.tsx) as
// values, from the node's report list (protocol 26, ADR 0042). Pure: no
// transport, no clock, no DOM; the caller supplies `now` and the language.
//
// [Decision Log] (2026-11-16, impl U, TRACKER 771)
// - Purpose: the page draws the same cards as the native app from the same
//   wire values, with Paseo's states: a current report has no status word,
//   a failed attempt reads "Error", a source that cannot report reads
//   "Unavailable"; a retained success stays visible beside either.
// - Constraint: the node owns which reports exist and which source may be
//   refreshed (its consent rules); the page never infers an account,
//   estimates a quota or shows a withheld (Operate-only) detail.
// - Choice: one projection function per report, reused by Settings and,
//   later, the sidebar summary and modal; words are English as the rest of
//   the page, numbers and dates follow the browser's language.
// - Cost: the English failure words (failures.ts) are copied from the native usage words
//   (view/locales/en/usage.ftl); a change there is made here too.
import type {
  CostVisibility,
  SourceReportEntry,
  SourceReportState,
  SuccessfulUsageReport,
  UsageBalance,
  UsageDetail,
  UsageReports,
  UsageSourceDescriptor,
  UsageWindow,
} from "../../wire/proto";
import { QuotaInstant, quotaClock, type QuotaClock } from "./clock";
import { FETCH, problemText } from "./failures";
import {
  QuotaPercent,
  quotaAmount,
  quotaPercent,
  quotaTone,
  type QuotaDisplay,
  type QuotaTone,
} from "./values";

/** The provider mark a card shows (the native `builtin_source_mark`):
 * Cursor, Grok, Kimi, MiniMax and Z.ai have no approved mark yet. */
export type QuotaMark = "claude" | "codex" | "opencode" | "github";

function markOf(builtin: SourceReportEntry["builtin"]): QuotaMark | null {
  switch (builtin) {
    case "claude":
      return "claude";
    case "codex":
      return "codex";
    case "copilot":
      return "github";
    case "open_code_go":
      return "opencode";
    case "cursor":
    case "grok":
    case "kimi":
    case "mini_max":
    case "zai":
    case null:
      return null;
  }
}

/** Paseo's status dot word; a current report shows none. */
export type QuotaCardStatus = "error" | "unavailable" | null;

export interface QuotaCardWindow {
  key: string;
  label: string;
  percent: QuotaPercent | null;
  tone: QuotaTone;
  clock: QuotaClock;
}

export interface QuotaCardBalance {
  key: string;
  label: string;
  /** "used / limit", "N left" or the used amount; Paseo's dash when unknown. */
  amount: string;
  /** The used share when an allowance is reported (Paseo meters only then). */
  percent: QuotaPercent | null;
  tone: QuotaTone;
  clock: QuotaClock;
}

export interface QuotaCardDetail {
  key: string;
  label: string;
  value: string;
}

export interface QuotaCard {
  key: string;
  report: SourceReportEntry["id"];
  source: SourceReportEntry["source"];
  builtin: SourceReportEntry["builtin"];
  mark: QuotaMark | null;
  name: string;
  plan: string | null;
  account: string | null;
  status: QuotaCardStatus;
  /** Why the last attempt did not report, in the page's words. */
  notes: readonly string[];
  /** Never fetched: Paseo's quiet "not checked yet" line. */
  notChecked: boolean;
  partial: boolean;
  windows: readonly QuotaCardWindow[];
  balances: readonly QuotaCardBalance[];
  details: readonly QuotaCardDetail[];
  /** The shown success's fetch time (the freshness tooltip). */
  fetchedAt: QuotaInstant | null;
  refreshing: boolean;
  /** Only an enabled source may be refreshed (ADR 0042). */
  refreshable: boolean;
}

const UNKNOWN = "—";

/** The success a state shows, and whether it is a retained one. */
function observedOf(state: SourceReportState): {
  observed: SuccessfulUsageReport | null;
  retained: boolean;
} {
  switch (state.state) {
    case "not_requested":
      return { observed: null, retained: false };
    case "available":
      return { observed: state.observed, retained: false };
    case "refreshing":
      // A pending refresh of an available report is still current.
      return {
        observed: state.retained,
        retained: state.origin === "unavailable",
      };
    case "unavailable":
    case "failed":
      return { observed: state.retained, retained: true };
  }
}

function instant(at: string | null): QuotaInstant | null {
  return at === null ? null : QuotaInstant.fromIso(at);
}

function windowOf(
  window: UsageWindow,
  display: QuotaDisplay,
  retained: boolean,
  now: QuotaInstant,
): QuotaCardWindow {
  const sides = { used: window.used, remaining: window.remaining };
  return {
    key: window.id,
    label: window.label,
    percent: quotaPercent(sides, display),
    tone: quotaTone(sides, window.tone),
    clock: quotaClock(
      {
        reset: instant(window.resets_at),
        runOut: instant(window.runs_out_at),
        shortfall:
          window.shortfall === null
            ? null
            : QuotaPercent.from(window.shortfall),
        basis: retained ? "retained" : "current",
      },
      now,
    ),
  };
}

/** Paseo balance-bar.tsx `resolveBalance`, amounts by `formatAmount`. */
export function balanceAmount(
  balance: UsageBalance,
  visibility: CostVisibility,
  locale: string,
): { amount: string; percent: QuotaPercent | null } {
  if (visibility === "withheld") {
    return { amount: "Hidden for this device", percent: null };
  }
  const text = (value: number): string => {
    const amount = quotaAmount(value, balance.unit, locale);
    return amount.kind === "reported" ? amount.text : UNKNOWN;
  };
  const { used, remaining, limit } = balance;
  if (limit !== null && limit > 0) {
    const usedAmount = used ?? (remaining === null ? null : limit - remaining);
    return {
      amount: `${usedAmount === null ? UNKNOWN : text(usedAmount)} / ${text(limit)}`,
      percent:
        usedAmount === null
          ? null
          : QuotaPercent.from((usedAmount / limit) * 100),
    };
  }
  if (remaining !== null) {
    return { amount: `${text(remaining)} left`, percent: null };
  }
  return { amount: used === null ? UNKNOWN : text(used), percent: null };
}

function balanceOf(
  balance: UsageBalance,
  visibility: CostVisibility,
  locale: string,
  retained: boolean,
  now: QuotaInstant,
): QuotaCardBalance {
  const { amount, percent } = balanceAmount(balance, visibility, locale);
  return {
    key: balance.id,
    label: balance.label,
    amount,
    percent,
    tone: balance.tone ?? "default",
    clock: quotaClock(
      {
        reset: instant(balance.resets_at),
        runOut: null,
        shortfall: null,
        basis: retained ? "retained" : "current",
      },
      now,
    ),
  };
}

function detailOf(detail: UsageDetail): QuotaCardDetail {
  return { key: detail.id, label: detail.label, value: detail.value };
}

function statusOf(
  state: SourceReportState,
  source: UsageSourceDescriptor | undefined,
): QuotaCardStatus {
  if (state.state === "failed") return "error";
  if (state.state === "unavailable") return "unavailable";
  return source?.availability === "unsupported" ? "unavailable" : null;
}

function notesOf(state: SourceReportState): string[] {
  if (state.state !== "unavailable" && state.state !== "failed") return [];
  const own =
    state.state === "failed" ? FETCH[state.reason] : problemText(state.problem);
  const logins = state.logins.map((login) => {
    const why =
      login.failure.kind === "error"
        ? FETCH[login.failure.reason]
        : problemText(login.failure.problem);
    return `${login.label}: ${why}`;
  });
  return [own, ...logins];
}

/** One card per report, in the node's order. */
export function quotaCards(
  reports: UsageReports,
  display: QuotaDisplay,
  locale: string,
  now: QuotaInstant,
): QuotaCard[] {
  return reports.reports.map((entry) => {
    const source = reports.sources.find((item) => item.id === entry.source);
    const { observed, retained } = observedOf(entry.state);
    const report = observed?.report ?? null;
    const visible = (detail: UsageDetail): boolean =>
      detail.access === "read" || reports.cost_visibility === "permitted";
    return {
      key: `${entry.source}/${entry.account}`,
      report: entry.id,
      source: entry.source,
      builtin: entry.builtin,
      mark: markOf(source?.builtin ?? entry.builtin),
      name: entry.source_label,
      plan: report?.plan ?? null,
      account: entry.account_label,
      status: statusOf(entry.state, source),
      notes: notesOf(entry.state),
      notChecked: entry.state.state === "not_requested",
      partial: report?.completeness === "partial",
      windows: (report?.windows ?? []).map((window) =>
        windowOf(window, display, retained, now),
      ),
      balances: (report?.balances ?? []).map((balance) =>
        balanceOf(balance, reports.cost_visibility, locale, retained, now),
      ),
      details: (report?.details ?? []).filter(visible).map(detailOf),
      fetchedAt: observed === null ? null : QuotaInstant.fromIso(observed.at),
      refreshing: entry.state.state === "refreshing",
      refreshable: source?.availability === "enabled",
    };
  });
}
