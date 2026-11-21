import { describe, expect, it } from "vitest";

import type {
  ReviewState,
  ThreadPlace,
  TimelineItem,
} from "../../../wire/proto";
import {
  activityCounts,
  ageOf,
  entries,
  entryFoldedAtFirst,
  entryKey,
  itemAge,
  shown,
  threadPath,
  trimmed,
} from "./timeline";

const BASE = Date.parse("2026-11-19T10:00:00Z");
const MINUTE = 60_000;
const at = (minute: number): string =>
  new Date(BASE + minute * MINUTE).toISOString();

function review(
  id: string,
  state: ReviewState,
  body: string,
  minute: number,
): TimelineItem {
  return {
    kind: "review",
    id,
    author: "reviewer",
    body,
    created_at: at(minute),
    state,
  };
}

function place(
  thread: string,
  resolved: boolean,
  outdated: boolean,
): ThreadPlace {
  return {
    thread,
    path: "src/lib.rs",
    line: 14,
    start_line: 12,
    resolved,
    outdated,
  };
}

function comment(
  id: string,
  body: string,
  minute: number,
  reviewId: string | null,
  thread: ThreadPlace | null,
): TimelineItem {
  return {
    kind: "comment",
    id,
    author: "author",
    body,
    created_at: at(minute),
    review: reviewId,
    thread,
  };
}

describe("the activity (timeline.rs)", () => {
  it("drops items that say nothing", () => {
    const items = [
      comment("c1", "  ", 0, null, null),
      review("r1", "commented", "", 1),
      review("r2", "approved", "", 2),
      review("r3", "commented", "Looks fine", 3),
    ];
    expect(
      shown(items).map((item) => item.id),
      "an approval without words still counts",
    ).toEqual(["r2", "r3"]);
  });

  it("gathers threads where they start and moves them under their review", () => {
    const open = place("t1", false, false);
    const loose = place("t2", true, false);
    const items = [
      comment("c0", "First", 0, null, null),
      review("r1", "changes_requested", "Please fix", 1),
      comment("c1", "Here", 2, "r1", open),
      comment("c2", "Elsewhere", 3, "gone", loose),
      comment("c3", "Done", 4, null, open),
      comment("c4", "Reply", 5, null, loose),
    ];
    const made = entries(shown(items));
    expect(made.map(entryKey)).toEqual(["c0", "r1", "thread:t2"]);
    const held = made[1];
    if (held?.kind !== "review") {
      throw new Error("the review holds its thread");
    }
    expect(
      held.threads[0]?.comments.map((item) => item.id),
      "a reply joins its thread",
    ).toEqual(["c1", "c3"]);
    const lone = made[2];
    if (lone?.kind !== "thread") {
      throw new Error("a thread whose review is not listed stays on top");
    }
    expect(lone.thread.comments.map((item) => item.id)).toEqual(["c2", "c4"]);
    expect(entryFoldedAtFirst(lone), "resolved starts folded").toBe(true);
    expect(made[0] !== undefined && entryFoldedAtFirst(made[0])).toBe(false);
    expect(entryFoldedAtFirst(held), "a review never").toBe(false);
  });

  it("keeps a review without threads a single card", () => {
    const made = entries(shown([review("r1", "approved", "", 0)]));
    expect(made[0]?.kind).toBe("single");
  });

  it("gives threads only to the first review of an id, as the native drain", () => {
    const items = [
      review("r1", "commented", "One", 0),
      review("r1", "commented", "Again", 1),
      comment("c1", "Here", 2, "r1", place("t1", false, false)),
    ];
    expect(entries(items).map((entry) => entry.kind)).toEqual([
      "review",
      "single",
    ]);
  });

  it("folds an outdated thread whose review is not listed", () => {
    const made = entries([
      comment("c1", "Old", 0, null, place("t9", false, true)),
    ]);
    expect(made[0] !== undefined && entryFoldedAtFirst(made[0])).toBe(true);
  });

  it("counts the flat list", () => {
    const thread = place("t1", false, true);
    const items = [
      review("r1", "approved", "", 0),
      review("r2", "changes_requested", "Fix", 1),
      review("r3", "commented", "Hm", 2),
      comment("c1", "Why?", 3, "r2", thread),
      comment("c2", "Because", 4, null, thread),
      comment("c3", "Thanks", 5, null, null),
    ];
    expect(activityCounts(shown(items))).toEqual({
      approvals: 1,
      changes: 1,
      comments: 4,
    });
  });

  it("steps ages as Paseo's", () => {
    const now = BASE;
    const ago = (minutes: number) => ageOf(now - minutes * MINUTE, now);
    expect(ageOf(now - 59_000, now)).toEqual({ step: "now" });
    expect(ageOf(now + 5 * MINUTE, now)).toEqual({ step: "now" });
    expect(ago(59)).toEqual({ step: "minutes", count: 59 });
    expect(ago(60)).toEqual({ step: "hours", count: 1 });
    expect(ago(24 * 60)).toEqual({ step: "days", count: 1 });
    expect(ago(29 * 24 * 60)).toEqual({ step: "days", count: 29 });
    expect(ago(65 * 24 * 60)).toEqual({ step: "months", count: 2 });
    expect(ago(400 * 24 * 60)).toEqual({ step: "years", count: 1 });
  });

  it("reads an item's age from its text, and a broken one as just now", () => {
    const item = comment("c1", "Hi", -3, null, null);
    expect(itemAge(item, BASE)).toEqual({ step: "minutes", count: 3 });
    expect(itemAge({ ...item, created_at: "whenever" }, BASE)).toEqual({
      step: "now",
    });
  });

  it("names a thread's lines", () => {
    const where = place("t1", false, false);
    expect(threadPath(where)).toBe("src/lib.rs:12-14");
    expect(threadPath({ ...where, start_line: null })).toBe("src/lib.rs:14");
    expect(threadPath({ ...where, start_line: null, line: null })).toBe(
      "src/lib.rs",
    );
    expect(threadPath({ ...where, path: null }), "a discussion thread").toBe(
      null,
    );
  });

  it("trims Unicode white space as Rust's str::trim does", () => {
    expect(trimmed("\u0085 hi 　")).toBe("hi");
    expect(trimmed("﻿hi"), "a BOM is not white space").toBe("﻿hi");
  });
});
