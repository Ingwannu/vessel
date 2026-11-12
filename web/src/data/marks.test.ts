import { describe, expect, test } from "vitest";

import { fixtureId } from "../testing/ids";
import type { SessionInfo } from "../wire/proto";
import { liveSession } from "./fixtures/machines";
import {
  canStartAgain,
  deletedLine,
  markActions,
  readThrough,
  type ReadView,
} from "./marks";

const ENDED: SessionInfo = {
  ...liveSession,
  status: { state: "exited", code: 0 },
  harness_session: fixtureId("HarnessSessionId", "h1"),
  abilities: { steer: false, resume: true, image: false, audio: false },
};

describe("what a session offers", () => {
  test("nothing on a machine before protocol 10", () => {
    expect(markActions(ENDED, 9)).toEqual([]);
  });

  test("a live session: pin, unread, archive; no delete or start again", () => {
    expect(markActions(liveSession, 10)).toEqual([
      "pin",
      "mark-unread",
      "archive",
    ]);
  });

  test("an ended one, archived, pinned and unread", () => {
    const marked: SessionInfo = {
      ...ENDED,
      marks: {
        pinned: true,
        unread: true,
        archived_at: "2026-11-12T00:00:00Z",
      },
    };
    expect(markActions(marked, 10)).toEqual([
      "unpin",
      "mark-read",
      "start-again",
      "unarchive",
      "delete",
    ]);
  });

  test("start again only where the agent can pick up its conversation", () => {
    expect(canStartAgain(ENDED)).toBe(true);
    expect(
      canStartAgain({
        ...ENDED,
        abilities: { steer: false, resume: false, image: false, audio: false },
      }),
    ).toBe(false);
    // Stored before protocol 10: no abilities recorded, the node decides.
    expect(canStartAgain({ ...ENDED, abilities: null })).toBe(true);
    // No agent session to load.
    expect(canStartAgain({ ...ENDED, harness_session: null })).toBe(false);
    expect(
      canStartAgain({
        ...ENDED,
        spec: {
          harness: "claude-code",
          mode: "terminal",
          args: [],
          size: { cols: 80, rows: 24 },
        },
      }),
    ).toBe(false);
  });
});

const FRONT: ReadView = {
  front: true,
  unread: false,
  attention: -1,
  next: 10,
  sent: -1,
  heldAt: null,
};

describe("when the page marks a session read", () => {
  test("only in front, and only after something asked for attention", () => {
    expect(readThrough(FRONT)).toBeNull();
    expect(readThrough({ ...FRONT, attention: 7 })).toBe(10);
    expect(readThrough({ ...FRONT, attention: 7, front: false })).toBeNull();
    expect(readThrough({ ...FRONT, unread: true })).toBe(10);
  });

  test("not again for what was already sent", () => {
    expect(readThrough({ ...FRONT, attention: 7, sent: 10 })).toBeNull();
    // A new turn's end after the last read.
    expect(readThrough({ ...FRONT, attention: 11, next: 12, sent: 10 })).toBe(
      12,
    );
  });

  test("after Mark unread, not until a record comes after it", () => {
    const held = { ...FRONT, unread: true, attention: 7, sent: 8, heldAt: 10 };
    expect(readThrough(held)).toBeNull();
    expect(readThrough({ ...held, next: 11, attention: 10 })).toBe(11);
  });
});

test("what became of the worktree, in words", () => {
  expect(deletedLine({})).toBeNull();
  expect(
    deletedLine({
      worktree: {
        worktree: fixtureId("WorktreeId", "w1"),
        outcome: "kept",
        reasons: [
          { kind: "uncommitted", files: 2 },
          { kind: "unique_commits", commits: 1 },
        ],
      },
    }),
  ).toBe(
    "Its worktree was kept: 2 uncommitted files, 1 commit no other branch has.",
  );
});
