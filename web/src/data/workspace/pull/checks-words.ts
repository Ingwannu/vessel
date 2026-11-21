// The checks section's words (Paseo's `checksOverview` messages): a
// group's label, a part of the detail line, the detail line itself and a
// running check's time. Mirrors the native
// `view/src/workspace/pull/checks_words.rs`.

import {
  duration,
  summaryParts,
  type ChecksSummary,
  type Presentation,
  type Timing,
} from "./checks";
import type { PullWords } from "./words";

/** One part of the detail line ("2 failing"). */
export function countWords(
  presentation: Presentation,
  count: number,
  words: PullWords,
): string {
  switch (presentation) {
    case "failure":
      return words.countFailing(count);
    case "pending":
      return words.countPending(count);
    case "success":
      return words.countPassed(count);
    case "ignored":
      return words.countSkipped(count);
  }
}

/** A group's label ("3 failing checks"). */
export function groupWords(
  presentation: Presentation,
  count: number,
  words: PullWords,
): string {
  switch (presentation) {
    case "failure":
      return words.groupFailing(count);
    case "pending":
      return words.groupPending(count);
    case "success":
      return words.groupPassed(count);
    case "ignored":
      return words.groupSkipped(count);
  }
}

/** The detail line ("2 failing, 4 in progress, 1 successful checks");
 * empty with no checks. */
export function detailWords(summary: ChecksSummary, words: PullWords): string {
  const parts = summaryParts(summary).map((part) =>
    countWords(part.presentation, part.count, words),
  );
  if (parts.length === 0) {
    return "";
  }
  return words.checksDetail(parts.join(", "), summary.total);
}

/** A row's time: the duration, or "running 2m" while it runs. */
export function timingWords(timing: Timing, words: PullWords): string {
  const spent = duration(timing.seconds);
  return timing.running ? words.checkRunning(spent) : spent;
}
