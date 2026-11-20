// The workspace hover card's diff stat (ADR 0120, TRACKER 819).
//
// [Decision Log]
// - Purpose: the sidebar card shows the served count after the pull
//   request, and follows it without a timer of its own.
// - Constraint: an older node ignores `changed_from` and answers at
//   once. A loop that sent the field anyway would spin.
// - Choice: the page asks only when the constant is a number and the
//   peer speaks at least that number. The comparison lives on a
//   `number | null` argument. The first read omits `changed_from`.
//   Later reads send the generation already shown. An empty list has no
//   row to carry the generation, so the last one is kept.

import type { DiffStat, PullSummary } from "../../wire/proto";

/** Whether a peer speaking `peer` is served the diff stat.
 *
 * `since` is `DIFF_STAT_PROTOCOL`. While that constant is `null`, this
 * is false for every peer. */
export function diffStatServed(since: number | null, peer: number): boolean {
  return since !== null && peer >= since;
}

/** The `changed_from` to send, or `null` to omit it.
 *
 * Omitted on the first read, and whenever the peer is not served the
 * stat. An older node would ignore the field and answer immediately. */
export function diffStatChangedFrom(
  since: number | null,
  peer: number,
  held: number | null,
): number | null {
  if (!diffStatServed(since, peer) || held === null) {
    return null;
  }
  return held;
}

/** The generation the next read waits on.
 *
 * Rows of one answer share it. An empty answer carries none, so the
 * generation already held stays. */
export function nextStatRevision(
  held: number | null,
  rows: readonly { stat_revision?: number | null }[],
): number | null {
  for (const row of rows) {
    const revision = row.stat_revision;
    if (typeof revision === "number") {
      return revision;
    }
  }
  return held;
}

/** Counts by workspace id. A missing `diff_stat` is null, which hides
 * the row. */
export function statsByWorkspace(
  rows: readonly { id: string; diff_stat?: DiffStat | null }[],
): ReadonlyMap<string, DiffStat | null> {
  const stats = new Map<string, DiffStat | null>();
  for (const row of rows) {
    stats.set(row.id, row.diff_stat ?? null);
  }
  return stats;
}

/** `1.2k`, as Paseo's diff count: en-US compact, one decimal, lowercase. */
export function formatDiffCount(value: number): string {
  return new Intl.NumberFormat("en-US", {
    notation: "compact",
    maximumFractionDigits: 1,
  })
    .format(value)
    .toLowerCase();
}

/** The branch every session that has one agrees on, or null. */
export function sharedBranch(
  branches: readonly (string | null | undefined)[],
): string | null {
  let agreed: string | null = null;
  for (const branch of branches) {
    if (branch === null || branch === undefined || branch === "") {
      continue;
    }
    if (agreed === null) {
      agreed = branch;
      continue;
    }
    if (agreed !== branch) {
      return null;
    }
  }
  return agreed;
}

/** A session's summary as the card compares them, or null. */
export function pullOf(
  summary: PullSummary | null | undefined,
): SharedPull | null {
  if (summary == null) {
    return null;
  }
  return {
    kind: summary.repo.kind,
    host: summary.repo.host,
    path: summary.repo.path,
    number: summary.number,
    checks: summary.checks,
  };
}

/** One pull request the workspace's summaries agree on. */
export interface SharedPull {
  kind: string;
  host: string;
  path: string;
  number: number;
  checks: string;
}

/** The pull request every summary agrees on, or null.
 *
 * Sessions with no summary are skipped. Agreement is the forge, the
 * repository, and the number. */
export function sharedPull(
  rows: readonly (SharedPull | null | undefined)[],
): SharedPull | null {
  let agreed: SharedPull | null = null;
  for (const row of rows) {
    if (row === null || row === undefined) {
      continue;
    }
    if (agreed === null) {
      agreed = row;
      continue;
    }
    if (
      agreed.kind !== row.kind ||
      agreed.host !== row.host ||
      agreed.path !== row.path ||
      agreed.number !== row.number
    ) {
      return null;
    }
  }
  return agreed;
}
