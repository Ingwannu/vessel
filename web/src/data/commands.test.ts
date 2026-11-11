import { describe, expect, it } from "vitest";

import type { BookNode } from "../wire/bridge";
import type { SessionInfo, SessionStatus } from "../wire/proto";
import { paletteEntries } from "./commands";
import type { FleetState } from "./fleet";

function session(
  id: string,
  at: string,
  status: SessionStatus,
  cwd: string,
): SessionInfo {
  return {
    id,
    spec: { harness: "claude-code", mode: "structured", cwd },
    created_by: { kind: "owner" },
    status,
    created_at: at,
    next_seq: 0,
  };
}

const laptop: BookNode = {
  key: "laptop",
  label: "laptop",
  address: "ws://127.0.0.1:7001/rpc",
  transport: "ws",
  default: true,
  id: null,
};

function state(): FleetState {
  return {
    link: { kind: "open", version: "0.1.0" },
    book: [laptop],
    bookError: null,
    statuses: {},
    harnesses: {},
    profiles: {},
    models: {},
    errors: {},
    selected: null,
    sessions: {
      laptop: [
        session(
          "aaaa1111",
          "2026-11-07T09:00:00Z",
          { state: "exited", code: 0 },
          "/w/old",
        ),
        session(
          "bbbb2222",
          "2026-11-07T10:00:00Z",
          { state: "running" },
          "/w/api",
        ),
        session(
          "cccc3333",
          "2026-11-07T11:00:00Z",
          { state: "running" },
          "/w/web",
        ),
      ],
    },
  };
}

describe("paletteEntries", () => {
  it("lists the action, the places, the nodes, then the sessions", () => {
    const groups = paletteEntries(state(), "laptop").map(
      (entry) => entry.group,
    );
    expect([...new Set(groups)]).toEqual([
      "Actions",
      "Go to",
      "Nodes",
      "Sessions",
    ]);
  });

  it("starts a new session on the selected node", () => {
    const action = paletteEntries(state(), "laptop").find(
      (entry) => entry.id === "new-session",
    );
    expect(action?.target).toEqual({ kind: "new-session", node: "laptop" });
  });

  it("puts live sessions first, newest first, and ended ones after", () => {
    const sessions = paletteEntries(state(), null)
      .filter((entry) => entry.group === "Sessions")
      .map((entry) => entry.label);
    expect(sessions).toEqual(["web cccc", "api bbbb", "old aaaa"]);
  });

  it("opens a session at its route and finds it by its full id", () => {
    const entry = paletteEntries(state(), null).find(
      (candidate) => candidate.keywords === "bbbb2222",
    );
    expect(entry?.target).toEqual({
      kind: "route",
      to: "/sessions/laptop/bbbb2222",
    });
    expect(entry?.detail).toBe("running · /w/api");
  });

  it("opens a node's page", () => {
    const entry = paletteEntries(state(), null).find(
      (candidate) => candidate.group === "Nodes",
    );
    expect(entry).toMatchObject({
      label: "laptop",
      target: { kind: "route", to: "/fleet/laptop" },
    });
  });
});
