import { describe, expect, it } from "vitest";

import { fixtureId } from "../../testing/ids";
import type {
  SessionInfo,
  TerminalInfo,
  TerminalList,
  TerminalPlace,
} from "../../wire/proto";
import type { Outcome } from "../outcome";
import {
  besideSessions,
  liveTerminalsIn,
  mayEndTerminals,
  readStrip,
  removalState,
  sessionHost,
  sessionInWorktree,
  shellOf,
  workspaceTerminalPlace,
} from "./terminal-place";
import { closesTerminals } from "./worktree-words";

const PROJECT = fixtureId("ProjectId", "p-shop");
const WORKTREE = fixtureId("WorktreeId", "w-fix");
const ROOT_PROJECT = { kind: "project" as const, project: PROJECT };
const ROOT_WORKTREE = { kind: "worktree" as const, worktree: WORKTREE };

function session(id: string, live: boolean, archived = false): SessionInfo {
  return {
    id: fixtureId("SessionId", id),
    spec: { harness: "claude-code", mode: "structured", cwd: "/work/notes" },
    created_by: { kind: "owner" },
    status: live ? { state: "running" } : { state: "exited", code: 0 },
    created_at: "2026-11-20T00:00:00Z",
    next_seq: 0,
    ...(archived ? { marks: { archived_at: "2026-11-20T01:00:00Z" } } : {}),
  };
}

function terminal(
  id: string,
  place: TerminalPlace,
  folder: string,
  live = true,
): TerminalInfo {
  return {
    id: fixtureId("TerminalId", id),
    place,
    folder,
    kind: { kind: "shell", program: "/bin/sh" },
    created_by: { kind: "owner" },
    created_at: "2026-11-20T00:00:00Z",
    status: live ? { state: "running" } : { state: "exited", code: 0 },
    oldest_seq: 0,
    next_seq: 0,
    revision: 1,
  };
}

describe("the place a workspace opens its shells in (ADR 0121)", () => {
  const host = fixtureId("SessionId", "s-live");

  it("is the project for a project workspace at every protocol", () => {
    for (const protocol of [39, 40]) {
      expect(
        workspaceTerminalPlace({
          protocol,
          root: ROOT_PROJECT,
          folder: "/work/shop",
          host,
        }),
      ).toEqual({ kind: "project", project: PROJECT });
    }
  });

  it("is the worktree, or the folder, from protocol 40, with no agent needed", () => {
    expect(
      workspaceTerminalPlace({
        protocol: 40,
        root: ROOT_WORKTREE,
        folder: "/work/shop-fix",
        host: null,
      }),
    ).toEqual({ kind: "worktree", worktree: WORKTREE });
    expect(
      workspaceTerminalPlace({
        protocol: 40,
        root: null,
        folder: "/work/notes",
        host: null,
      }),
    ).toEqual({ kind: "folder", path: "/work/notes" });
  });

  it("stays beside a session below 40, and is none when every agent is archived", () => {
    expect(
      workspaceTerminalPlace({
        protocol: 39,
        root: ROOT_WORKTREE,
        folder: "/work/shop-fix",
        host,
      }),
    ).toEqual({ kind: "session", session: host });
    expect(
      workspaceTerminalPlace({
        protocol: 39,
        root: null,
        folder: "/work/notes",
        host: null,
      }),
    ).toBeNull();
  });

  it("falls back to a session for a folder the page cannot name", () => {
    expect(
      workspaceTerminalPlace({ protocol: 40, root: null, folder: null, host }),
    ).toEqual({ kind: "session", session: host });
  });

  it("puts a session's shells beside the first live unarchived agent", () => {
    expect(
      sessionHost([
        session("s-archived", true, true),
        session("s-ended", false),
        session("s-live", true),
      ]),
    ).toBe(fixtureId("SessionId", "s-live"));
    expect(sessionHost([session("s-ended", false)])).toBe(
      fixtureId("SessionId", "s-ended"),
    );
    expect(sessionHost([session("s-archived", true, true)])).toBeNull();
  });
});

describe("which listed shells are the workspace's", () => {
  const sessions = new Set([fixtureId("SessionId", "s-1")]);

  it("matches a worktree by id and a project by its folder", () => {
    const place: TerminalPlace = { kind: "worktree", worktree: WORKTREE };
    expect(
      shellOf(place, terminal("t-1", place, "/work/shop-fix"), sessions),
    ).toBe(true);
    expect(
      shellOf(
        place,
        terminal(
          "t-2",
          { kind: "worktree", worktree: fixtureId("WorktreeId", "w-other") },
          "/work/shop-other",
        ),
        sessions,
      ),
    ).toBe(false);
    const project: TerminalPlace = { kind: "project", project: PROJECT };
    expect(
      shellOf(project, terminal("t-3", project, "/work/shop"), sessions),
    ).toBe(true);
    expect(
      shellOf(
        project,
        terminal(
          "t-4",
          { kind: "project", project: PROJECT, dir: "web" },
          "/work/shop/web",
        ),
        sessions,
      ),
    ).toBe(false);
  });

  it("finds the shells beside the workspace's agents, and only theirs", () => {
    const beside: TerminalPlace = {
      kind: "session",
      session: fixtureId("SessionId", "s-1"),
    };
    const elsewhere: TerminalPlace = {
      kind: "session",
      session: fixtureId("SessionId", "s-9"),
    };
    const found = besideSessions(
      [
        terminal("t-agent", beside, "/work/notes"),
        terminal("t-other", elsewhere, "/work/other"),
        terminal(
          "t-own",
          { kind: "folder", path: "/work/notes" },
          "/work/notes",
        ),
      ],
      sessions,
    );
    expect(found.map((item) => item.id)).toEqual(["t-agent"]);
  });
});

/** A node whose lists change from one tick to the next: `ticks[n]` answers
 * the n-th tick's place read and its read of every terminal. */
function node(ticks: readonly { own: TerminalInfo[]; all: TerminalInfo[] }[]): {
  list: (place: TerminalPlace | null) => Promise<Outcome<TerminalList>>;
  next: () => void;
  asked: (TerminalPlace | null)[];
} {
  let tick = 0;
  const asked: (TerminalPlace | null)[] = [];
  return {
    asked,
    next: () => {
      tick += 1;
    },
    list: (place) => {
      asked.push(place);
      const now = ticks[Math.min(tick, ticks.length - 1)];
      const terminals =
        now === undefined ? [] : place === null ? now.all : now.own;
      return Promise.resolve({ ok: true, value: { terminals, revision: 1 } });
    },
  };
}

describe("one tick of the strip (the node reviewer's R1 on TRACKER 829)", () => {
  const sessions = new Set([fixtureId("SessionId", "s-1")]);
  const worktree: TerminalPlace = { kind: "worktree", worktree: WORKTREE };
  const beside: TerminalPlace = {
    kind: "session",
    session: fixtureId("SessionId", "s-1"),
  };
  const own = terminal("t-own", worktree, "/w/fix");

  it("shows a shell an agent opens after the workspace opened, on the next tick", async () => {
    const agentShell = terminal("t-agent", beside, "/w/fix");
    const fake = node([
      { own: [own], all: [own] },
      { own: [own], all: [own, agentShell] },
    ]);
    const first = await readStrip(fake.list, worktree, sessions);
    expect(first.ok ? first.value.map((item) => item.id) : null).toEqual([
      "t-own",
    ]);
    fake.next();
    const second = await readStrip(fake.list, worktree, sessions);
    expect(second.ok ? second.value.map((item) => item.id) : null).toEqual([
      "t-own",
      "t-agent",
    ]);
    // Each tick reads the place and every terminal, side by side.
    expect(fake.asked).toEqual([worktree, null, worktree, null]);
  });

  it("carries a shell that ended as ended, and drops one that is gone", async () => {
    const running = terminal("t-agent", beside, "/w/fix");
    const ended = terminal("t-agent", beside, "/w/fix", false);
    const fake = node([
      { own: [own], all: [own, running] },
      { own: [own], all: [own, ended] },
      { own: [own], all: [own] },
    ]);
    await readStrip(fake.list, worktree, sessions);
    fake.next();
    const second = await readStrip(fake.list, worktree, sessions);
    // The strip keeps live shells as tabs and closes the ended ones'
    // (use-workspace: `isLiveTerminal`), so the ended state must come
    // through.
    expect(
      second.ok
        ? second.value.find((item) => item.id === "t-agent")?.status.state
        : null,
    ).toBe("exited");
    fake.next();
    const third = await readStrip(fake.list, worktree, sessions);
    expect(third.ok ? third.value.map((item) => item.id) : null).toEqual([
      "t-own",
    ]);
  });

  it("keeps the last list when either read fails", async () => {
    const failing = (place: TerminalPlace | null) =>
      Promise.resolve<Outcome<TerminalList>>(
        place === null
          ? {
              ok: false,
              problem: { from: "op", reason: "busy", message: "busy" },
            }
          : { ok: true, value: { terminals: [own], revision: 1 } },
      );
    const read = await readStrip(failing, worktree, sessions);
    expect(read.ok).toBe(false);
  });

  it("reads a project's place, or a session's, once a tick", async () => {
    const project: TerminalPlace = { kind: "project", project: PROJECT };
    const fake = node([{ own: [], all: [] }]);
    await readStrip(fake.list, project, sessions);
    await readStrip(fake.list, beside, sessions);
    expect(fake.asked).toEqual([project, null]);
  });
});

describe("a worktree's removal and its terminals (ADR 0121)", () => {
  it("counts the live terminals in the worktree's folder, whatever their place", () => {
    const listed = [
      terminal("t-1", { kind: "worktree", worktree: WORKTREE }, "/w/fix"),
      terminal(
        "t-2",
        { kind: "session", session: fixtureId("SessionId", "s-1") },
        "/w/fix/web",
      ),
      terminal(
        "t-3",
        { kind: "worktree", worktree: WORKTREE },
        "/w/fix",
        false,
      ),
      terminal("t-4", { kind: "folder", path: "/w/fix2" }, "/w/fix2"),
    ];
    expect(liveTerminalsIn(listed, "/w/fix")).toBe(2);
  });

  it("asks the node to end them only when no live session is in it, from 40", () => {
    const asked = {
      protocol: 40,
      inUse: true,
      liveTerminals: 2,
      sessionInIt: false,
    };
    expect(mayEndTerminals(asked)).toBe(true);
    expect(mayEndTerminals({ ...asked, sessionInIt: true })).toBe(false);
    expect(mayEndTerminals({ ...asked, liveTerminals: 0 })).toBe(false);
    expect(mayEndTerminals({ ...asked, inUse: false })).toBe(false);
    expect(mayEndTerminals({ ...asked, protocol: 39 })).toBe(false);
  });

  it("finds a live session in the worktree as the node does, by folder too", () => {
    const at = (
      cwd: string,
      live: boolean,
      inWorktree = false,
    ): SessionInfo => ({
      ...session("s-cli", live),
      spec: { harness: "claude-code", mode: "structured", cwd },
      ...(inWorktree
        ? { workspace: { project: PROJECT, worktree: WORKTREE } }
        : {}),
    });
    const fix = { id: WORKTREE, path: "/w/fix" };
    // Started by folder (the CLI's --cwd), with no workspace on it.
    expect(sessionInWorktree([at("/w/fix/web", true)], fix)).toBe(true);
    expect(sessionInWorktree([at("/w/fix", true)], fix)).toBe(true);
    expect(sessionInWorktree([at("/w/fix2", true)], fix)).toBe(false);
    expect(sessionInWorktree([at("/w/fix", false)], fix)).toBe(false);
    expect(sessionInWorktree([at("/elsewhere", true, true)], fix)).toBe(true);
  });

  it("decides what the dialog offers while counting, and after", () => {
    const base = { protocol: 40, inUse: true, sessionInIt: false };
    // Still counting: nothing said yet, the buttons wait.
    expect(removalState({ ...base, liveTerminals: null })).toEqual({
      checking: true,
      endTerminals: false,
      blocked: true,
    });
    // Counted: the buttons end them first.
    expect(removalState({ ...base, liveTerminals: 2 })).toEqual({
      checking: false,
      endTerminals: true,
      blocked: false,
    });
    // None counted: in_use is setup or a session; held as today.
    expect(removalState({ ...base, liveTerminals: 0 })).toEqual({
      checking: false,
      endTerminals: false,
      blocked: true,
    });
    // A live session in it: held, whatever was counted.
    expect(
      removalState({ ...base, sessionInIt: true, liveTerminals: 2 }),
    ).toEqual({ checking: false, endTerminals: false, blocked: true });
    // Below 40: as before 0121.
    expect(
      removalState({ ...base, protocol: 39, liveTerminals: null }),
    ).toEqual({ checking: false, endTerminals: false, blocked: true });
    // Nothing in use: nothing to hold or end.
    expect(removalState({ ...base, inUse: false, liveTerminals: 0 })).toEqual({
      checking: false,
      endTerminals: false,
      blocked: false,
    });
  });

  it("says Paseo's terminal warning with the count", () => {
    expect(closesTerminals(2)).toBe(
      "This will close 2 terminal(s). Any running process in a closed terminal will be stopped immediately.",
    );
  });
});
