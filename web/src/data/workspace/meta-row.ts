// The line under a workspace's name in the sidebar, and the row's name
// (TRACKER 426 on the page): Paseo's `WorkspaceMetaRow`
// (`components/sidebar/workspace-meta-row/`, pinned 0.15.4) and its
// `resolveSidebarWorkspaceAccessibilityLabel`
// (`sidebar-workspace-title.ts` 16-37).
//
// [Decision Log]
// - Purpose: a workspace row says where it lives, its pull request and
//   how that pull request's checks stand, without a hover, as Paseo's.
// - Constraint: the node sends the summary's checks state and, since
//   protocol 39, its tally (passed, failed, pending), not every check's
//   status; the page's sidebar words are English.
// - Choice: the rules here, pure and tested; the markup is
//   `features/shell/workspace-meta.tsx`. Items in Paseo's order: the
//   machine (only in a fleet of several, Paseo's host badge), the pull
//   request, its checks, the labels. The checks' outcome is the
//   summary's state (a failure wins over progress); the running pie's
//   quarters are passed and failed of the tally's total, rounded down
//   (Paseo's `toFilledQuarters`); without a tally the ring stays empty
//   (Paseo's `summarizeChecksStatus`: running reads 0 of 1).
// - Differences: a state the node does not know shows the open glyph in
//   the muted colour with no word (Paseo calls anything not open or
//   merged closed); the row's name has no status part, since a page
//   workspace row has none (its sessions carry theirs).

import type { SharedPull } from "./diff-stat";

/** Paseo's badge states (`pr-hint.ts`): a draft is open. */
export type PullBadge = "open" | "merged" | "closed" | "unknown";

/** Paseo's `CheckSummaryState`. */
export type ChecksOutcome = "passed" | "failed" | "running";

/** Quarters of the running pie that are filled. */
export type Quarters = 0 | 1 | 2 | 3 | 4;

export interface CheckSummary {
  outcome: ChecksOutcome;
  quarters: Quarters;
}

export type MetaItem =
  | { kind: "host"; name: string }
  | { kind: "changeRequest"; pull: SharedPull; badge: PullBadge }
  | { kind: "checks"; summary: CheckSummary }
  | { kind: "labels" };

/** The badge for `pull`'s state. */
export function badgeOf(pull: SharedPull): PullBadge {
  switch (pull.state) {
    case "open":
    case "merged":
    case "closed":
      return pull.state;
    case "unknown":
      return "unknown";
  }
}

/** Paseo's state word after the number: none while open. */
export function badgeWord(badge: PullBadge): string | null {
  switch (badge) {
    case "merged":
      return "merged";
    case "closed":
      return "closed";
    case "open":
    case "unknown":
      return null;
  }
}

/** The checks as one glyph and word, or null when there are none. */
export function checkSummary(pull: SharedPull | null): CheckSummary | null {
  if (pull === null) {
    return null;
  }
  switch (pull.checks) {
    case "failing":
    case "cancelled":
      return { outcome: "failed", quarters: 4 };
    case "passing":
      return { outcome: "passed", quarters: 4 };
    case "pending":
      return { outcome: "running", quarters: quartersOf(pull) };
    default:
      return null;
  }
}

/** Done (passed or failed) of the tally's total, in whole quarters
 * rounded down: nine of ten is three quarters; only the tenth closes it. */
function quartersOf(pull: SharedPull): Quarters {
  const counts = pull.counts;
  if (counts === null) {
    return 0;
  }
  const done = counts.passed + counts.failed;
  const total = done + counts.pending;
  if (total <= 0) {
    return 0;
  }
  return QUARTERS[Math.min(4, Math.floor((done * 4) / total))] ?? 4;
}

const QUARTERS: readonly Quarters[] = [0, 1, 2, 3, 4];

/** Paseo's one word after the ring (`checksSummary.*Label`). */
export const CHECK_WORDS: Record<ChecksOutcome, string> = {
  passed: "passed",
  failed: "failed",
  running: "running",
};

/** The checks item's name (`checksSummary.*Accessible`). */
export const CHECK_NAMES: Record<ChecksOutcome, string> = {
  passed: "Checks passed",
  failed: "Checks failed",
  running: "Checks running",
};

/** A pull request's name (`workspace.git.pr.accessibility.pullRequest`). */
export function pullName(number: number): string {
  return `Pull request #${String(number)}`;
}

/** What the line carries, in Paseo's order. `machine`: its name only in a
 * fleet of several. Empty: no line. */
export function metaItems(input: {
  machine: string | null;
  pull: SharedPull | null;
  labels: number;
}): MetaItem[] {
  const items: MetaItem[] = [];
  if (input.machine !== null) {
    items.push({ kind: "host", name: input.machine });
  }
  if (input.pull !== null) {
    items.push({
      kind: "changeRequest",
      pull: input.pull,
      badge: badgeOf(input.pull),
    });
  }
  const summary = checkSummary(input.pull);
  if (summary !== null) {
    items.push({ kind: "checks", summary });
  }
  if (input.labels > 0) {
    items.push({ kind: "labels" });
  }
  return items;
}

/** The row's name: its title, its machine where shown, its pull request,
 * joined by a comma (Paseo's order, the parts a page row has). */
export function workspaceName(input: {
  title: string;
  machine: string | null;
  pull: SharedPull | null;
}): string {
  const parts = [input.title];
  if (input.machine !== null) {
    parts.push(input.machine);
  }
  if (input.pull !== null) {
    parts.push(pullName(input.pull.number));
  }
  return parts.filter((part) => part !== "").join(", ");
}
