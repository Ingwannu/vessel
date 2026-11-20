import { describe, expect, it } from "vitest";

import { fixtureId } from "../../testing/ids";
import type {
  SessionInfo,
  TerminalInfo,
  TerminalPlace,
} from "../../wire/proto";
import {
  liveTerminalsIn,
  mayEndTerminals,
  olderShells,
  sessionHost,
  shellOf,
  stripShells,
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

  it("finds the shells opened beside the workspace's agents before the upgrade", () => {
    const beside: TerminalPlace = {
      kind: "session",
      session: fixtureId("SessionId", "s-1"),
    };
    const elsewhere: TerminalPlace = {
      kind: "session",
      session: fixtureId("SessionId", "s-9"),
    };
    const found = olderShells(
      [
        terminal("t-old", beside, "/work/notes"),
        terminal("t-other", elsewhere, "/work/other"),
        terminal(
          "t-new",
          { kind: "folder", path: "/work/notes" },
          "/work/notes",
        ),
      ],
      sessions,
    );
    expect(found.map((item) => item.id)).toEqual(["t-old"]);
  });

  it("adds the older shells the opening read found to the strip", () => {
    const folder: TerminalPlace = { kind: "folder", path: "/work/notes" };
    const shown = terminal("t-new", folder, "/work/notes");
    const placed = [shown];
    const older = [
      terminal(
        "t-old",
        { kind: "session", session: fixtureId("SessionId", "s-1") },
        "/work/notes",
      ),
    ];
    expect(stripShells(placed, older).map((item) => item.id)).toEqual([
      "t-new",
      "t-old",
    ]);
    // Listed by the place too (renamed, say), it is shown once.
    expect(
      stripShells(placed, [shown, ...older]).map((item) => item.id),
    ).toEqual(["t-new", "t-old"]);
  });

  it("is the place's list alone when the opening read found none", () => {
    const folder: TerminalPlace = { kind: "folder", path: "/work/notes" };
    const placed = [terminal("t-new", folder, "/work/notes")];
    expect(stripShells(placed, [])).toBe(placed);
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

  it("says Paseo's terminal warning with the count", () => {
    expect(closesTerminals(2)).toBe(
      "This will close 2 terminal(s). Any running process in a closed terminal will be stopped immediately.",
    );
  });
});
