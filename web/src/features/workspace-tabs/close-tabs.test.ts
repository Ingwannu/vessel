import { describe, expect, it } from "vitest";

import { fixtureId } from "../../testing/ids";
import type {
  SessionInfo,
  TerminalInfo,
  TerminalPlace,
} from "../../wire/proto";
import { endedByArchive } from "./close-tabs";

const AGENT = fixtureId("SessionId", "s-agent");

const archiving = new Map<SessionInfo["id"], SessionInfo>([
  [
    AGENT,
    {
      id: AGENT,
      spec: { harness: "claude-code", mode: "structured", cwd: "/w/fix" },
      created_by: { kind: "owner" },
      status: { state: "running" },
      created_at: "2026-11-20T00:00:00Z",
      next_seq: 0,
    },
  ],
]);

function shell(id: string, place: TerminalPlace): TerminalInfo {
  return {
    id: fixtureId("TerminalId", id),
    place,
    folder: "/w/fix",
    kind: { kind: "shell", program: "/bin/sh" },
    created_by: { kind: "owner" },
    created_at: "2026-11-20T00:00:00Z",
    status: { state: "running" },
    oldest_seq: 0,
    next_seq: 0,
    revision: 1,
  };
}

describe("what an agent's archive ends (ADR 0121, TRACKER 829)", () => {
  it("counts the shells beside the agent and never a worktree's or a folder's", () => {
    const listed = [
      shell("t-beside", { kind: "session", session: AGENT }),
      shell("t-worktree", {
        kind: "worktree",
        worktree: fixtureId("WorktreeId", "w-fix"),
      }),
      shell("t-folder", { kind: "folder", path: "/w/fix" }),
      shell("t-project", {
        kind: "project",
        project: fixtureId("ProjectId", "p-shop"),
      }),
    ];
    expect(endedByArchive(listed, archiving)).toEqual([
      fixtureId("TerminalId", "t-beside"),
    ]);
  });
});
