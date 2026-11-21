// A pull request's facts as Paseo's pull request pane reads them
// (`git/pull-request-panel/pane.tsx`: the state under the title;
// `checks-summary.ts`: the checks' headline and their counts in the order
// a person acts on them; the activity summary's approval and change
// request pills), from what the node sends. Mirrors the native
// `view/src/workspace/pull/facts.rs`.
//
// [Decision Log]
// - Purpose: one reading of the facts for the pane's header, tested once.
// - Constraint: the node sends counts (passed, failed, pending) and the
//   failing checks (at most twenty), and every run only from
//   `PULL_REQUEST_PANE_PROTOCOL` on: without the runs, Paseo's groups of
//   successful and skipped checks cannot be listed by name.
// - Choice: closed unions in Paseo's order; counts as numbers. A value
//   the node does not know is "unknown", never passing or mergeable.
// - Differences: none in the rules. The unions are string literals in
//   the page's camelCase where the native ones are enums.

import type {
  Checks,
  ChecksState,
  Mergeable,
  PullSummary,
  ReviewDecision,
  Reviews,
} from "../../../wire/proto";

/** Where it is in its life, as the state line under its title names it. */
export type PullPhase = "open" | "draft" | "merged" | "closed" | "unknown";

/** The phase of `summary`: only an open one can be a draft. */
export function phaseOf(summary: PullSummary): PullPhase {
  switch (summary.state) {
    case "open":
      return summary.draft ? "draft" : "open";
    case "merged":
    case "closed":
    case "unknown":
      return summary.state;
  }
}

/** The worst thing happening in the checks: Paseo's `ChecksOutcome`, the
 * headline and the ring's colour. "unknown" is not read yet. */
export type ChecksOutcome =
  "failure" | "pending" | "success" | "none" | "unknown";

/** The outcome the node's summary names: a cancelled run is a failure. */
export function outcomeOf(state: ChecksState): ChecksOutcome {
  switch (state) {
    case "failing":
    case "cancelled":
      return "failure";
    case "pending":
      return "pending";
    case "passing":
      return "success";
    case "none":
      return "none";
    case "unknown":
      return "unknown";
  }
}

/** One count of checks the node tallied. */
export interface CheckCount {
  readonly kind: "failing" | "pending" | "passed";
  readonly count: number;
}

/** The counts to show without the runs, in Paseo's order (what to act
 * on, then what runs, then what passed); only counts above zero. */
export function checkCounts(checks: Checks): CheckCount[] {
  const all: readonly CheckCount[] = [
    { kind: "failing", count: checks.failed },
    { kind: "pending", count: checks.pending },
    { kind: "passed", count: checks.passed },
  ];
  return all.filter((count) => count.count > 0);
}

/** The review verdict as a person reads it. */
export type Verdict =
  "approved" | "changesRequested" | "reviewRequired" | "none" | "unknown";

/** The forge's decision. */
export function verdictOf(decision: ReviewDecision): Verdict {
  switch (decision) {
    case "approved":
      return "approved";
    case "changes_requested":
      return "changesRequested";
    case "review_required":
      return "reviewRequired";
    case "none":
      return "none";
    case "unknown":
      return "unknown";
  }
}

/** Paseo's activity pills: approvals and change requests that stand, and
 * unresolved threads (the node counts threads, not comments). */
export interface ReviewPills {
  readonly approved: boolean;
  readonly changesRequested: boolean;
  readonly unresolved: number;
}

/** The pills of `reviews`. */
export function reviewPillsOf(reviews: Reviews): ReviewPills {
  return {
    approved: reviews.approved,
    changesRequested: reviews.changes_requested,
    unresolved: reviews.unresolved,
  };
}

/** The forge's verdict, unless a pill already names the same fact. The
 * node may send a summary decision that differs from the reviews'
 * standing, so hiding every verdict would lose information. */
export function remainingVerdict(
  pills: ReviewPills,
  decision: ReviewDecision,
): Verdict {
  const verdict = verdictOf(decision);
  if (verdict === "approved" && pills.approved) {
    return "none";
  }
  if (verdict === "changesRequested" && pills.changesRequested) {
    return "none";
  }
  return verdict;
}

/** Whether it can merge now, as a person reads it. */
export type Merge =
  "ready" | "conflicts" | "blocked" | "unstable" | "behind" | "unknown";

/** The forge's mergeability. */
export function mergeOf(mergeable: Mergeable): Merge {
  switch (mergeable) {
    case "mergeable":
      return "ready";
    case "conflicting":
      return "conflicts";
    case "blocked":
      return "blocked";
    case "unstable":
      return "unstable";
    case "behind":
      return "behind";
    case "unknown":
      return "unknown";
  }
}

/** Readiness, for an open request that is not a draft; null otherwise.
 * State, draft and mergeability are separate wire facts: a draft cannot
 * claim readiness even when the forge reports its head mergeable, and a
 * closed or merged request may keep a mergeability from before. */
export function mergeForOpen(summary: PullSummary): Merge | null {
  return summary.state === "open" && !summary.draft
    ? mergeOf(summary.mergeable)
    : null;
}
