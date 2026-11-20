// Every check of a pull request as Paseo's checks section reads it
// (`git/check-presentation.ts`, `pull-request-panel/checks-summary.ts`,
// `checks-ring.tsx`, and the server's duration in `github-service.ts`
// 4141-4175), from the runs a node at `PULL_REQUEST_PANE_PROTOCOL` sends
// (ADR 0088 part 2). Mirrors the native
// `view/src/workspace/pull/checks.rs`.
//
// [Decision Log]
// - Purpose: the pane's headline, its ring, its groups and each row's
//   timing, decided once and tested outside the screen.
// - Constraint: GitHub's checks carry no traits, so Paseo's
//   `actionRequired`, `warning` and `manual` presentations never occur
//   (they are GitLab's and Gitea's); an older node sends no runs, only
//   counts and the failing ones (`checkCounts` in facts.ts).
// - Choice: the four presentations GitHub reaches, in Paseo's order;
//   `ChecksOutcome` for the headline; the ring as dash lengths in the
//   ring's own units; a timing as whole seconds, worded by the language
//   (checks-words.ts). `now` is a parameter, so a test fixes it.
// - Differences: lengths are doubles, not the native f32 (they differ
//   below 1e-5 of a unit, which no screen draws); the native share clamps
//   counts to 65,535 to convert them to f32, which a double does not need.
//   A time the wire sends that does not parse gives no timing.

import type { CheckRun, CheckState } from "../../../wire/proto";
import type { ChecksOutcome } from "./facts";
import { instant } from "./time";

/** How a check is drawn and grouped (Paseo's `CheckPresentation`, the
 * values GitHub reaches): failed; runs or waits (and a state the page
 * does not know); passed; skipped or cancelled, which says nothing about
 * the run. */
export type Presentation = "failure" | "pending" | "success" | "ignored";

/** Paseo's `PRESENTATION_ORDER`: what to act on, then what runs, then what
 * needs no attention, skipped last. */
export const PRESENTATION_ORDER: readonly Presentation[] = [
  "failure",
  "pending",
  "success",
  "ignored",
];

/** Paseo's `classifyCheck` for a check with no traits. */
export function presentationOf(state: CheckState): Presentation {
  switch (state) {
    case "passed":
      return "success";
    case "failed":
      return "failure";
    case "cancelled":
    case "skipped":
      return "ignored";
    case "pending":
    case "unknown":
      return "pending";
  }
}

/** The checks of one presentation, in the order they came. */
export interface Group {
  readonly presentation: Presentation;
  readonly checks: readonly CheckRun[];
}

/** One presentation's count, for the detail line and the ring. */
export interface Part {
  readonly presentation: Presentation;
  readonly count: number;
}

/** Everything the checks section draws (Paseo's `ChecksSummary`). */
export interface ChecksSummary {
  /** The headline's outcome. */
  readonly outcome: ChecksOutcome;
  /** The non-empty groups, in `PRESENTATION_ORDER`. */
  readonly groups: readonly Group[];
  /** Every check, skipped ones included. */
  readonly total: number;
}

/** `runs` summarized: a failure outranks anything still running, and
 * checks that were all skipped read as passed, as Paseo's do. */
export function checksSummary(runs: readonly CheckRun[]): ChecksSummary {
  const groups = PRESENTATION_ORDER.flatMap((presentation): Group[] => {
    const checks = runs.filter(
      (run) => presentationOf(run.state) === presentation,
    );
    return checks.length === 0 ? [] : [{ presentation, checks }];
  });
  const has = (presentation: Presentation): boolean =>
    groups.some((group) => group.presentation === presentation);
  let outcome: ChecksOutcome = "success";
  if (runs.length === 0) {
    outcome = "none";
  } else if (has("failure")) {
    outcome = "failure";
  } else if (has("pending")) {
    outcome = "pending";
  }
  return { outcome, groups, total: runs.length };
}

/** Each group's presentation and count. */
export function summaryParts(summary: ChecksSummary): Part[] {
  return summary.groups.map((group) => ({
    presentation: group.presentation,
    count: group.checks.length,
  }));
}

/** The ring's box (Paseo's `ICON_SIZE.lg`). */
export const RING_SIZE = 20;
/** The ring's stroke. */
export const RING_STROKE = 3;
/** The ring's radius: half the box less half the stroke, (20 - 3) / 2. */
export const RING_RADIUS = 8.5;
/** The gap between two segments, along the ring. */
const SEGMENT_GAP = 2;
/** The shortest segment drawn. */
const MIN_SEGMENT = 1.5;

/** One coloured stretch of the ring, from twelve o'clock clockwise, in
 * the ring's own length units. */
export interface Segment {
  /** Its colour's presentation. */
  readonly presentation: Presentation;
  /** Where it starts along the ring. */
  readonly start: number;
  /** Its length. */
  readonly length: number;
}

/** The ring's circumference. */
export function circumference(): number {
  return 2 * Math.PI * RING_RADIUS;
}

/** Paseo's ring: one segment per part, each its share of the ring less a
 * gap (never shorter than the minimum); one part fills the ring. The next
 * segment starts after the whole share, so the gaps stay even. */
export function segments(parts: readonly Part[], total: number): Segment[] {
  if (total <= 0) {
    return [];
  }
  const whole = circumference();
  const single = parts.length === 1;
  let start = 0;
  return parts.map((part) => {
    const arc = (part.count / total) * whole;
    const length = single ? whole : Math.max(arc - SEGMENT_GAP, MIN_SEGMENT);
    const segment = { presentation: part.presentation, start, length };
    start += arc;
    return segment;
  });
}

/** How long a check ran, as its row says it. */
export interface Timing {
  /** It still runs ("running 2m"). */
  readonly running: boolean;
  /** Whole seconds from its start to its end, or to now while it runs. */
  readonly seconds: number;
}

/** A check's timing at `now` (milliseconds since the epoch): none without
 * a start, or with an end before it. */
export function timing(run: CheckRun, now: number): Timing | null {
  const started = instant(run.started_at);
  if (started === null) {
    return null;
  }
  const ended =
    run.completed_at === undefined || run.completed_at === null
      ? now
      : instant(run.completed_at);
  if (ended === null) {
    return null;
  }
  // chrono's `num_seconds` truncates toward zero: an end under a second
  // before its start is still 0 s, and a whole second before it is none.
  const seconds = Math.trunc((ended - started) / 1000);
  if (seconds < 0) {
    return null;
  }
  return {
    running: presentationOf(run.state) === "pending",
    // `Math.max` turns the -0 of a truncated small negative into 0.
    seconds: Math.max(0, seconds),
  };
}

/** Paseo's server's duration: hours, minutes and seconds, zero hours and
 * minutes left out, seconds kept when nothing else is ("1h 5m", "1m 30s",
 * "45s", "0s"). The same in every language, as Paseo's. */
export function duration(seconds: number): string {
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const rest = seconds % 60;
  const parts: string[] = [];
  if (hours > 0) {
    parts.push(`${hours}h`);
  }
  if (minutes > 0) {
    parts.push(`${minutes}m`);
  }
  if (rest > 0 || parts.length === 0) {
    parts.push(`${rest}s`);
  }
  return parts.join(" ");
}
