// Terminal notices on the page (ADR 0114 "Terminal notices"): a
// connection's first list tells nothing; a later one tells each Finished or
// Needs-input attention at a revision not told before, never a failed
// terminal's; a view clears only the exact revision shown; a click finds a
// session whose workspace holds the terminal.

import { describe, expect, test } from "vitest";

import { fixtureId } from "../testing/ids";
import type {
  SessionInfo,
  SessionStatus,
  SessionWorkspace,
  TerminalAttentionReason,
  TerminalId,
  TerminalInfo,
} from "../wire/proto";
import {
  holds,
  readTerminals,
  terminalNoticeTitle,
  terminalSource,
  toClear,
} from "./terminal-notices";
import { requestTerminalTab, takeTerminalTab } from "./terminal-tab-request";

const LAB = fixtureId("NodeKey", "lab");
const T1 = fixtureId("TerminalId", "t1");
const T2 = fixtureId("TerminalId", "t2");
const S1 = fixtureId("SessionId", "s1");

function terminal(
  id: TerminalId,
  attention: {
    revision: number;
    reason: TerminalAttentionReason | null;
  } | null,
  extra: { status?: SessionStatus; title?: string | null } = {},
): TerminalInfo {
  return {
    id,
    place: { kind: "session", session: S1 },
    folder: "/work",
    kind: { kind: "shell", program: "/bin/sh" },
    title: extra.title === undefined ? "build" : extra.title,
    created_by: { kind: "owner" },
    created_at: "2026-11-21T00:00:00Z",
    status: extra.status ?? { state: "running" },
    oldest_seq: 0,
    next_seq: 0,
    revision: 1,
    attention:
      attention === null
        ? null
        : {
            terminal: id,
            revision: attention.revision,
            activity:
              attention.reason === null
                ? { state: "working" }
                : { state: "attention", reason: attention.reason },
            changed_at: "2026-11-21T00:00:00Z",
          },
  };
}

function session(workspace: SessionWorkspace | null): SessionInfo {
  return {
    id: S1,
    spec: { harness: "claude-code", mode: "structured", cwd: "/work" },
    created_by: { kind: "owner" },
    status: { state: "running" },
    created_at: "2026-11-21T00:00:00Z",
    next_seq: 0,
    workspace,
  };
}

describe("terminal notices, page side", () => {
  test("the first list tells nothing; a new revision is told once", () => {
    const first = readTerminals(null, LAB, [
      terminal(T1, { revision: 3, reason: "finished" }),
    ]);
    expect(first.notices).toEqual([]);
    const same = readTerminals(first.watched, LAB, [
      terminal(T1, { revision: 3, reason: "finished" }),
    ]);
    expect(same.notices).toEqual([]);
    const next = readTerminals(same.watched, LAB, [
      terminal(T1, { revision: 4, reason: "needs_input" }),
    ]);
    expect(next.notices).toEqual([
      {
        kind: "needs-you",
        node: LAB,
        terminal: T1,
        revision: 4,
        place: { kind: "session", session: S1 },
        title: "build",
      },
    ]);
    expect(next.notices.map(terminalSource)).toEqual([
      { target: "terminal", terminal: T1, revision: 4 },
    ]);
    expect(terminalNoticeTitle("needs-you")).toBe("Terminal needs input");
    expect(terminalNoticeTitle("finished")).toBe("Terminal finished");
  });

  test("only finished or needs input of a terminal that did not fail is told", () => {
    const { watched } = readTerminals(null, LAB, []);
    const read = readTerminals(watched, LAB, [
      terminal(T1, { revision: 2, reason: null }),
      terminal(T2, { revision: 2, reason: "unknown" }),
      terminal(
        fixtureId("TerminalId", "t3"),
        { revision: 2, reason: "finished" },
        {
          status: { state: "failed", message: "gone" },
        },
      ),
      terminal(
        fixtureId("TerminalId", "t4"),
        { revision: 5, reason: "finished" },
        {
          title: null,
        },
      ),
    ]);
    expect(
      read.notices.map((each) => [each.terminal, each.kind, each.title]),
    ).toEqual([[fixtureId("TerminalId", "t4"), "finished", ""]]);
  });

  test("a view clears only attention at the exact revision shown", () => {
    expect(
      toClear(terminal(T1, { revision: 7, reason: "needs_input" })),
    ).toEqual({
      terminal: T1,
      revision: 7,
    });
    expect(toClear(terminal(T1, { revision: 8, reason: null }))).toBeNull();
    expect(toClear(terminal(T1, null))).toBeNull();
  });

  test("a click finds a session whose workspace holds the terminal's place", () => {
    const project = fixtureId("ProjectId", "p1");
    const worktree = fixtureId("WorktreeId", "w1");
    expect(holds(session(null), { kind: "session", session: S1 })).toBe(true);
    expect(holds(session(null), { kind: "folder", path: "/work" })).toBe(true);
    expect(holds(session({ project }), { kind: "project", project })).toBe(
      true,
    );
    expect(
      holds(session({ project, worktree }), { kind: "worktree", worktree }),
    ).toBe(true);
    expect(
      holds(session({ project }), { kind: "folder", path: "/work" }),
      "a rooted session's workspace is its root, not its folder",
    ).toBe(false);
  });

  test("a click's tab request is taken once, by a workspace that lists the terminal", () => {
    requestTerminalTab(LAB, T1);
    expect(takeTerminalTab(fixtureId("NodeKey", "box"), [T1])).toBeNull();
    expect(takeTerminalTab(LAB, [T2])).toBeNull();
    expect(takeTerminalTab(LAB, [T2, T1])).toBe(T1);
    expect(takeTerminalTab(LAB, [T1])).toBeNull();
  });
});
