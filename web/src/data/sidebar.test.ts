import { describe, expect, it } from "vitest";

import type { SessionInfo } from "../wire/proto";
import type { FleetState } from "./fleet";
import {
  DEFAULT_VIEW,
  sessionMark,
  sidebarMachines,
  sidebarOrder,
  sidebarStatuses,
  stepSession,
} from "./sidebar";
import { fixtureId } from "../testing/ids";

function session(
  id: string,
  at: string,
  extra: Partial<SessionInfo> = {},
): SessionInfo {
  return {
    id: fixtureId("SessionId", id),
    spec: { harness: "claude-code", mode: "structured", cwd: `/work/${id}` },
    created_by: { kind: "owner" },
    status: { state: "running" },
    created_at: at,
    next_seq: 0,
    ...extra,
  };
}

const STATE: FleetState = {
  link: { kind: "open", version: "0.1.0" },
  book: [
    {
      key: fixtureId("NodeKey", "laptop"),
      label: "laptop",
      address: "ws://laptop",
      transport: "ws",
      default: true,
      id: null,
    },
    {
      key: fixtureId("NodeKey", "gpu-box"),
      label: "gpu-box",
      address: "ws://gpu-box",
      transport: "ws",
      default: false,
      id: null,
    },
  ],
  bookError: null,
  statuses: {
    [fixtureId("NodeKey", "laptop")]: {
      state: "connected",
      protocol: 9,
      node: {
        id: fixtureId("NodeId", "n-1"),
        name: "laptop",
        hostname: "laptop",
        os: "linux",
        arch: "x86_64",
        version: "0.1.0",
        protocol: { min: 2, max: 9 },
        started_at: "2026-11-11T06:00:00Z",
        state: "serving",
      },
    },
  },
  harnesses: {},
  profiles: {},
  models: {},
  agents: {},
  sessions: {
    [fixtureId("NodeKey", "laptop")]: [
      session("aaaa", "2026-11-11T08:00:00Z", { title: "Fix the overview" }),
      session("bbbb", "2026-11-11T09:00:00Z"),
      session("cccc", "2026-11-11T07:00:00Z", {
        status: { state: "exited", code: 0 },
      }),
    ],
  },
  errors: {},
  selected: null,
};

describe("sidebarMachines", () => {
  it("lists each machine with its live sessions, newest first, by title", () => {
    const [laptop, box] = sidebarMachines(STATE, {
      ...DEFAULT_VIEW,
      ended: false,
    });
    expect(laptop).toMatchObject({
      key: "laptop",
      connected: true,
      word: "Connected",
      live: 2,
      more: 0,
    });
    expect(laptop?.sessions.map((row) => row.title)).toEqual([
      "bbbb",
      "Fix the overview",
    ]);
    expect(box).toMatchObject({ connected: false, live: 0, sessions: [] });
  });

  it("links to the board for the rest", () => {
    const [laptop] = sidebarMachines(
      STATE,
      { ...DEFAULT_VIEW, ended: false },
      1,
    );
    expect(laptop?.sessions).toHaveLength(1);
    expect(laptop?.more).toBe(1);
  });
});

describe("the view the person chose", () => {
  it("lists ended sessions when asked, and sorts by activity", () => {
    const view = {
      group: "machine",
      sort: "activity",
      ended: true,
      usage: false,
    } as const;
    const [laptop] = sidebarMachines(STATE, view);
    expect(laptop?.sessions.map((row) => [row.id, row.mark])).toEqual([
      ["bbbb", "idle"],
      ["aaaa", "idle"],
      ["cccc", "ended"],
    ]);
  });

  it("groups by status across machines, leaving empty groups out", () => {
    const groups = sidebarStatuses(STATE, {
      ...DEFAULT_VIEW,
      group: "status",
      ended: false,
    });
    expect(groups.map((group) => group.label)).toEqual(["Idle"]);
    expect(groups[0]?.sessions.map((row) => row.machine)).toEqual([
      "laptop",
      "laptop",
    ]);
  });

  it("walks the sessions in the order shown, wrapping", () => {
    const order = sidebarOrder(STATE, { ...DEFAULT_VIEW, ended: false });
    expect(order.map((row) => row.id)).toEqual(["bbbb", "aaaa"]);
    expect(stepSession(order, { node: "laptop", id: "aaaa" }, 1)?.id).toBe(
      "bbbb",
    );
    expect(stepSession(order, { node: "laptop", id: "bbbb" }, -1)?.id).toBe(
      "aaaa",
    );
    expect(stepSession(order, null, -1)?.id).toBe("aaaa");
    expect(stepSession([], null, 1)).toBeNull();
  });
});

describe("sessionMark", () => {
  it("is idle for a session of a node that does not place it", () => {
    expect(sessionMark(session("aaaa", "2026-11-11T08:00:00Z"))).toBe("idle");
  });
});

describe("the marks a machine keeps (protocol 10)", () => {
  it("leaves archived sessions out, lists pinned first, says unread", () => {
    const laptop = fixtureId("NodeKey", "laptop");
    const marked: FleetState = {
      ...STATE,
      sessions: {
        [laptop]: [
          session("aaaa", "2026-11-11T08:00:00Z"),
          session("bbbb", "2026-11-11T09:00:00Z", {
            marks: { archived_at: "2026-11-11T10:00:00Z" },
          }),
          session("dddd", "2026-11-11T06:00:00Z", {
            marks: { pinned: true, unread: true },
          }),
        ],
      },
    };
    const rows = sidebarMachines(marked, DEFAULT_VIEW)[0]?.sessions ?? [];
    expect(rows.map((row) => row.id)).toEqual(["dddd", "aaaa"]);
    expect(rows[0]).toMatchObject({ pinned: true, unread: true });
    expect(rows[1]).toMatchObject({ pinned: false, unread: false });
  });
});
