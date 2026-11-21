import { describe, expect, it } from "vitest";

import type {
  Checks,
  Mergeable,
  PullState,
  PullSummary,
  ReviewDecision,
} from "../../../wire/proto";
import {
  checkCounts,
  mergeForOpen,
  mergeOf,
  outcomeOf,
  phaseOf,
  remainingVerdict,
  reviewPillsOf,
  verdictOf,
  type Merge,
  type Verdict,
} from "./facts";

/** Number 42 of acme/widgets, in `state` (the native fixture). */
function summary(
  state: PullState,
  draft: boolean,
  mergeable: Mergeable = "mergeable",
): PullSummary {
  return {
    repo: { kind: "github", host: "github.com", path: "acme/widgets" },
    number: 42,
    url: "https://github.com/acme/widgets/pull/42",
    title: "Add the widget",
    state,
    draft,
    head_commit: "0123456789abcdef0123456789abcdef01234567",
    checks: "passing",
    review: "none",
    mergeable,
    updated_at: "2026-11-14T09:00:00Z",
  };
}

function checks(passed: number, failed: number, pending: number): Checks {
  return {
    state: "failing",
    passed,
    failed,
    pending,
    truncated: false,
    failing: [],
    runs: [],
  };
}

describe("a pull request's facts (facts.rs)", () => {
  it("reads an open draft as a draft and a closed one never", () => {
    expect(phaseOf(summary("open", false))).toBe("open");
    expect(phaseOf(summary("open", true))).toBe("draft");
    expect(phaseOf(summary("merged", true))).toBe("merged");
    expect(phaseOf(summary("closed", true))).toBe("closed");
    expect(phaseOf(summary("unknown", false))).toBe("unknown");
  });

  it("reads a cancelled check as a failure and unknown never as success", () => {
    expect(outcomeOf("cancelled")).toBe("failure");
    expect(outcomeOf("failing")).toBe("failure");
    expect(outcomeOf("pending")).toBe("pending");
    expect(outcomeOf("passing")).toBe("success");
    expect(outcomeOf("none")).toBe("none");
    expect(outcomeOf("unknown")).toBe("unknown");
  });

  it("counts failing, then running, then passed, only above zero", () => {
    expect(checkCounts(checks(5, 2, 0))).toEqual([
      { kind: "failing", count: 2 },
      { kind: "passed", count: 5 },
    ]);
    expect(checkCounts(checks(0, 0, 0))).toEqual([]);
  });

  it("follows the forge's verdict, mergeability and pills", () => {
    expect(verdictOf("changes_requested")).toBe("changesRequested");
    expect(verdictOf("unknown")).toBe("unknown");
    expect(mergeOf("conflicting")).toBe("conflicts");
    expect(mergeOf("unknown")).toBe("unknown");
    expect(
      reviewPillsOf({
        approved: true,
        changes_requested: false,
        commented: true,
        unresolved: 3,
      }),
    ).toEqual({ approved: true, changesRequested: false, unresolved: 3 });
  });

  it("removes only the verdict a pill already names", () => {
    for (const approved of [false, true]) {
      for (const changesRequested of [false, true]) {
        const pills = { approved, changesRequested, unresolved: 2 };
        const cases: [ReviewDecision, Verdict][] = [
          ["approved", approved ? "none" : "approved"],
          ["changes_requested", changesRequested ? "none" : "changesRequested"],
          ["review_required", "reviewRequired"],
          ["none", "none"],
          ["unknown", "unknown"],
        ];
        for (const [decision, expected] of cases) {
          expect(remainingVerdict(pills, decision)).toBe(expected);
        }
      }
    }
  });

  it("never claims merge readiness for a draft or a request not open", () => {
    const cases: [Mergeable, Merge][] = [
      ["mergeable", "ready"],
      ["conflicting", "conflicts"],
      ["blocked", "blocked"],
      ["unstable", "unstable"],
      ["behind", "behind"],
      ["unknown", "unknown"],
    ];
    const closedStates: PullState[] = ["merged", "closed", "unknown"];
    for (const [mergeable, expected] of cases) {
      for (const state of closedStates) {
        for (const draft of [false, true]) {
          expect(mergeForOpen(summary(state, draft, mergeable))).toBeNull();
        }
      }
      expect(mergeForOpen(summary("open", false, mergeable))).toBe(expected);
      expect(mergeForOpen(summary("open", true, mergeable))).toBeNull();
    }
  });
});
