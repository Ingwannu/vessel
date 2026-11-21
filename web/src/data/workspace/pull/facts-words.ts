// The pane header's facts in words, chosen from their closed unions:
// mirrors the native `view/src/workspace/pull/words.rs` for what the pane
// draws (the state, the checks' headline and counts, the verdict, the
// merge standing, why there is none). A forge's message or a host is a
// value in a sentence the translator owns, never a sentence of its own.

import type {
  CheckCount,
  ChecksOutcome,
  Merge,
  PullPhase,
  Verdict,
} from "./facts";
import type { Unavailable } from "./unavailable";
import type { PullWords } from "./words";

/** The state beside its icon. */
export function phaseWords(phase: PullPhase, words: PullWords): string {
  switch (phase) {
    case "open":
      return words.stateOpen;
    case "draft":
      return words.stateDraft;
    case "merged":
      return words.stateMerged;
    case "closed":
      return words.stateClosed;
    case "unknown":
      return words.stateUnknown;
  }
}

/** The checks' headline (Paseo's `checksOverview.headline`). */
export function outcomeWords(outcome: ChecksOutcome, words: PullWords): string {
  switch (outcome) {
    case "failure":
      return words.checksFailure;
    case "pending":
      return words.checksPending;
    case "success":
      return words.checksSuccess;
    case "none":
      return words.checksNone;
    case "unknown":
      return words.checksUnknown;
  }
}

/** One count without the runs (Paseo's `checksOverview.count`). */
export function checkCountWords(count: CheckCount, words: PullWords): string {
  switch (count.kind) {
    case "failing":
      return words.countFailing(count.count);
    case "pending":
      return words.countPending(count.count);
    case "passed":
      return words.countPassed(count.count);
  }
}

/** The review verdict; null when there is none to say. */
export function verdictWords(
  verdict: Verdict,
  words: PullWords,
): string | null {
  switch (verdict) {
    case "approved":
      return words.approved;
    case "changesRequested":
      return words.changesRequested;
    case "reviewRequired":
      return words.reviewRequired;
    case "none":
    case "unknown":
      return null;
  }
}

/** Whether it can merge now. */
export function mergeWords(merge: Merge, words: PullWords): string {
  switch (merge) {
    case "ready":
      return words.mergeReady;
    case "conflicts":
      return words.mergeConflicts;
    case "blocked":
      return words.mergeBlocked;
    case "unstable":
      return words.mergeUnstable;
    case "behind":
      return words.mergeBehind;
    case "unknown":
      return words.mergeUnknown;
  }
}

/** Why there is no pull request, whole (Paseo's Info toast sentences). */
export function unavailableWords(why: Unavailable, words: PullWords): string {
  switch (why.kind) {
    case "noBranch":
      return words.noBranch;
    case "noRemote":
      return words.noRemote;
    case "notSupported":
      return words.noForge(why.host);
    case "signedOut":
      return words.signedOut(why.host);
    case "refused":
      return words.refused(why.host);
    case "rateLimited":
      return words.rateLimited(why.host);
    case "unreachable":
      return words.unreachable(why.host);
    case "limit":
      return words.limit;
    case "ended":
      return words.ended;
    case "unknown":
      return words.unavailable;
  }
}
