import { describe, expect, it } from "vitest";

import { DIFF_STAT_PROTOCOL, type PullSummary } from "../../wire/proto";
import {
  diffStatChangedFrom,
  diffStatServed,
  formatDiffCount,
  nextStatRevision,
  pullOf,
  sharedBranch,
  sharedPull,
  statsByWorkspace,
} from "./diff-stat";

describe("the served diff stat (ADR 0120)", () => {
  it("serves a peer at 38 and not an older one", () => {
    expect(DIFF_STAT_PROTOCOL).toBe(38);
    expect(diffStatServed(DIFF_STAT_PROTOCOL, 37)).toBe(false);
    expect(diffStatServed(DIFF_STAT_PROTOCOL, 38)).toBe(true);
    expect(diffStatServed(null, 38)).toBe(false);
  });

  it("omits changed_from for an older peer and for the first read", () => {
    expect(diffStatChangedFrom(null, 36, 4)).toBeNull();
    expect(diffStatChangedFrom(37, 36, 4)).toBeNull();
    expect(diffStatChangedFrom(37, 37, null)).toBeNull();
    expect(diffStatChangedFrom(37, 37, 4)).toBe(4);
  });

  it("keeps the generation when a list answer has no row", () => {
    expect(nextStatRevision(3, [])).toBe(3);
    expect(nextStatRevision(null, [])).toBeNull();
    expect(
      nextStatRevision(3, [{ stat_revision: 5 }, { stat_revision: 5 }]),
    ).toBe(5);
  });

  it("keeps a null count so the row stays hidden", () => {
    const stats = statsByWorkspace([
      { id: "a", diff_stat: { additions: 2, deletions: 1 } },
      { id: "b" },
    ]);
    expect(stats.get("a")).toEqual({ additions: 2, deletions: 1 });
    expect(stats.get("b")).toBeNull();
  });

  it("writes compact lowercase counts", () => {
    expect(formatDiffCount(0)).toBe("0");
    expect(formatDiffCount(999)).toBe("999");
    expect(formatDiffCount(1200)).toBe("1.2k");
    expect(formatDiffCount(1_000_000)).toBe("1m");
  });

  it("shows a branch only when every named one agrees", () => {
    expect(sharedBranch([null, undefined, ""])).toBeNull();
    expect(sharedBranch([null, "main", undefined])).toBe("main");
    expect(sharedBranch(["feature", "feature"])).toBe("feature");
    expect(sharedBranch(["main", "feature"])).toBeNull();
  });

  it("shows one pull request only when the summaries agree", () => {
    const one = {
      kind: "github",
      host: "github.com",
      path: "team/app",
      number: 12,
      checks: "passing",
      counts: null,
      url: null,
    };
    expect(sharedPull([null, undefined])).toBeNull();
    expect(sharedPull([null, one])).toEqual(one);
    expect(sharedPull([one, { ...one, checks: "failing" }])).toEqual(one);
    expect(sharedPull([one, { ...one, number: 13 }])).toBeNull();
  });

  it("reads check counts only when the summary carries a tally", () => {
    const summary: PullSummary = {
      repo: { kind: "github", host: "github.com", path: "team/app" },
      number: 12,
      url: "https://github.com/team/app/pull/12",
      title: "Keyboard order",
      state: "open",
      draft: false,
      head_commit: "a".repeat(40),
      checks: "passing",
      review: "approved",
      mergeable: "mergeable",
      updated_at: "2026-11-20T00:00:00Z",
    };
    expect(pullOf(summary)?.counts).toBeNull();
    const counted: PullSummary = {
      ...summary,
      tally: { passed: 3, failed: 1, pending: 2 },
    };
    expect(pullOf(counted)?.counts).toEqual({
      passed: 3,
      failed: 1,
      pending: 2,
    });
    expect(pullOf(counted)?.url).toBe("https://github.com/team/app/pull/12");
  });
});
