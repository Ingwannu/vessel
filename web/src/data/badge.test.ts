// The badge and the favicon, Paseo's rule over Vessel's sessions (ADR 0089,
// "The page's badge and favicon"): a workspace takes its most pressing
// session's bucket, the badge counts workspaces in needs input, failed or
// attention, and the favicon is running, attention or none.

import { describe, expect, test } from "vitest";

import { fixtureId } from "../testing/ids";
import type { NodeKey } from "../wire/bridge";
import type {
  SessionInfo,
  SessionMarks,
  SessionStatus,
  SessionWorkspace,
} from "../wire/proto";
import {
  badgeCount,
  bucketOf,
  faviconHref,
  faviconStatus,
  workspaceBuckets,
} from "./badge";

const LAB = fixtureId("NodeKey", "lab");
const PROJECT = fixtureId("ProjectId", "p1");

function session(
  id: string,
  options: {
    status?: SessionStatus;
    blocked?: boolean;
    marks?: SessionMarks;
    workspace?: SessionWorkspace | null;
  } = {},
): SessionInfo {
  return {
    id: fixtureId("SessionId", id),
    spec: { harness: "claude-code", mode: "structured", cwd: "/work" },
    created_by: { kind: "owner" },
    status: options.status ?? { state: "exited", code: 0 },
    created_at: "2026-11-21T00:00:00Z",
    next_seq: 0,
    board:
      options.blocked === true
        ? {
            column: "building",
            display_status: "blocked",
            activity: "waiting_input",
            last_activity_at: "2026-11-21T00:00:00Z",
          }
        : null,
    marks: options.marks ?? {},
    workspace: options.workspace ?? null,
  };
}

function on(...list: SessionInfo[]): Record<NodeKey, SessionInfo[]> {
  return { [LAB]: list };
}

describe("the badge and the favicon (Paseo's buckets)", () => {
  test("each session falls in Paseo's bucket", () => {
    expect(
      bucketOf(session("a", { status: { state: "running" }, blocked: true })),
    ).toBe("needs_input");
    expect(
      bucketOf(session("b", { status: { state: "failed", message: "x" } })),
    ).toBe("failed");
    expect(bucketOf(session("c", { status: { state: "running" } }))).toBe(
      "running",
    );
    expect(bucketOf(session("d", { marks: { unread: true } }))).toBe(
      "attention",
    );
    expect(bucketOf(session("e"))).toBe("done");
  });

  test("a workspace takes its most pressing session; the badge counts workspaces", () => {
    const sessions = on(
      // One project workspace: a running and a waiting session, so it
      // needs input, once.
      session("a", {
        status: { state: "running" },
        workspace: { project: PROJECT },
      }),
      session("b", {
        status: { state: "running" },
        blocked: true,
        workspace: { project: PROJECT },
      }),
      // Three rootless sessions, each its own workspace.
      session("c", { marks: { unread: true } }),
      session("d", { status: { state: "failed", message: "x" } }),
      session("e"),
      // Archived: off the sidebar, counted for nothing.
      session("f", {
        marks: { unread: true, archived_at: "2026-11-21T00:00:00Z" },
      }),
    );
    const buckets = workspaceBuckets(sessions);
    expect([...buckets].sort()).toEqual(
      ["attention", "done", "failed", "needs_input"].sort(),
    );
    expect(badgeCount(buckets)).toBe(3);
    expect(badgeCount(workspaceBuckets(on(session("e"))))).toBe(0);
  });

  test("the favicon is running, else attention, else none", () => {
    expect(
      faviconStatus(
        on(
          session("a", { status: { state: "running" } }),
          session("b", { marks: { unread: true } }),
        ),
      ),
    ).toBe("running");
    expect(faviconStatus(on(session("b", { marks: { unread: true } })))).toBe(
      "attention",
    );
    expect(
      faviconStatus(
        on(session("d", { status: { state: "failed", message: "x" } })),
      ),
      "a failed session turns no favicon, as in Paseo",
    ).toBe("none");
    expect(faviconHref("none")).toBe("/favicon.svg");
    expect(faviconHref("running")).toBe("/favicon-running.svg");
    expect(faviconHref("attention")).toBe("/favicon-attention.svg");
  });
});
