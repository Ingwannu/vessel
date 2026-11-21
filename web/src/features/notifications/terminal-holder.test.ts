// Which session a terminal notice's click opens (app review of claims row 3,
// note 2; node review of the follow-ups, note 4): a session that holds the
// place as a path first, with no call; otherwise each folder session's folder
// is listed in turn, a failed or empty one passed over, and the first that
// lists the terminal is the holder.

import { describe, expect, test } from "vitest";

import { fixtureId } from "../../testing/ids";
import type {
  SessionInfo,
  SessionWorkspace,
  TerminalId,
  TerminalInfo,
  TerminalPlace,
} from "../../wire/proto";
import type { TerminalNotice } from "../../data/terminal-notices";
import { holderOf } from "./terminal-holder";

const LAB = fixtureId("NodeKey", "lab");
const T1 = fixtureId("TerminalId", "t1");
const T2 = fixtureId("TerminalId", "t2");

function session(
  id: string,
  cwd: string,
  workspace: SessionWorkspace | null = null,
): SessionInfo {
  return {
    id: fixtureId("SessionId", id),
    spec: { harness: "claude-code", mode: "structured", cwd },
    created_by: { kind: "owner" },
    status: { state: "running" },
    created_at: "2026-11-21T00:00:00Z",
    next_seq: 0,
    workspace,
  };
}

function shell(id: TerminalId): TerminalInfo {
  return {
    id,
    place: { kind: "folder", path: "/real/notes" },
    folder: "/real/notes",
    kind: { kind: "shell", program: "/bin/sh" },
    title: "build",
    created_by: { kind: "owner" },
    created_at: "2026-11-21T00:00:00Z",
    status: { state: "running" },
    oldest_seq: 0,
    next_seq: 0,
    revision: 1,
    attention: null,
  };
}

function notice(place: TerminalPlace): TerminalNotice {
  return {
    kind: "finished",
    node: LAB,
    terminal: T1,
    revision: 2,
    place,
    title: "build",
  };
}

describe("a terminal notice's holder", () => {
  test("a session that holds the place as a path wins without a call", async () => {
    const asked: string[] = [];
    const held = await holderOf(
      [session("s1", "/real/notes")],
      notice({ kind: "folder", path: "/real/notes" }),
      (path) => {
        asked.push(path);
        return Promise.resolve(null);
      },
    );
    expect(held).toBe(fixtureId("SessionId", "s1"));
    expect(asked).toEqual([]);
  });

  test("a folder no session names is asked of the node, folder by folder", async () => {
    const project = fixtureId("ProjectId", "p1");
    const asked: string[] = [];
    const lists: Record<string, readonly TerminalInfo[] | null> = {
      "/gone": null,
      "/other": [shell(T2)],
      "/home/me/notes": [shell(T1)],
    };
    const held = await holderOf(
      [
        session("rooted", "/real/notes", { project }),
        session("gone", "/gone"),
        session("other", "/other"),
        session("link", "/home/me/notes"),
        session("after", "/after"),
      ],
      notice({ kind: "folder", path: "/real/notes" }),
      (path) => {
        asked.push(path);
        return Promise.resolve(lists[path] ?? []);
      },
    );
    expect(held).toBe(fixtureId("SessionId", "link"));
    expect(asked).toEqual(["/gone", "/other", "/home/me/notes"]);
  });

  test("another place no session holds opens nothing, with no call", async () => {
    const asked: string[] = [];
    const held = await holderOf(
      [session("s1", "/work")],
      notice({ kind: "worktree", worktree: fixtureId("WorktreeId", "w1") }),
      (path) => {
        asked.push(path);
        return Promise.resolve(null);
      },
    );
    expect(held).toBeNull();
    expect(asked).toEqual([]);
  });
});
