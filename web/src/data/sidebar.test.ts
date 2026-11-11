import { describe, expect, it } from "vitest";

import type { SessionInfo } from "../wire/proto";
import type { FleetState } from "./fleet";
import { sessionMark, sidebarMachines } from "./sidebar";

function session(
  id: string,
  at: string,
  extra: Partial<SessionInfo> = {},
): SessionInfo {
  return {
    id,
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
      key: "laptop",
      label: "laptop",
      address: "ws://laptop",
      transport: "ws",
      default: true,
      id: null,
    },
    {
      key: "gpu-box",
      label: "gpu-box",
      address: "ws://gpu-box",
      transport: "ws",
      default: false,
      id: null,
    },
  ],
  bookError: null,
  statuses: {
    laptop: {
      state: "connected",
      protocol: 9,
      node: {
        id: "n-1",
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
  sessions: {
    laptop: [
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
    const [laptop, box] = sidebarMachines(STATE);
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
    const [laptop] = sidebarMachines(STATE, 1);
    expect(laptop?.sessions).toHaveLength(1);
    expect(laptop?.more).toBe(1);
  });
});

describe("sessionMark", () => {
  it("is idle for a session of a node that does not place it", () => {
    expect(sessionMark(session("aaaa", "2026-11-11T08:00:00Z"))).toBe("idle");
  });
});
