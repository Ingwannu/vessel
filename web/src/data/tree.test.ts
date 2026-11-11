import { describe, expect, it } from "vitest";

import type { Creator, SessionInfo, SessionStatus } from "../wire/proto";
import type { FleetState } from "./fleet";
import { tree } from "./tree";

function session(
  id: string,
  at: string,
  status: SessionStatus,
  created_by: Creator,
): SessionInfo {
  return {
    id,
    spec: { harness: "claude-code", mode: "structured" },
    created_by,
    status,
    created_at: at,
    next_seq: 0,
  };
}

const running: SessionStatus = { state: "running" };
const exited: SessionStatus = { state: "exited", code: 0 };
const owner: Creator = { kind: "owner" };
const agentOf = (parent: string, node: string): Creator => ({
  kind: "agent",
  node,
  session: parent,
});

/** A person's session on laptop whose agent started one on gpu-box, which
 * started one on build-server (DESIGN.md §2.4), and an unrelated one. */
function state(parentStatus: SessionStatus): FleetState {
  return {
    link: { kind: "open", version: "0.1.0" },
    book: null,
    bookError: null,
    statuses: {},
    harnesses: {},
    profiles: {},
    models: {},
    errors: {},
    selected: null,
    sessions: {
      laptop: [
        session("3f2a", "2026-11-07T10:00:00Z", parentStatus, owner),
        session("9aa0", "2026-11-07T11:00:00Z", running, owner),
      ],
      "gpu-box": [
        session(
          "a91c",
          "2026-11-07T10:01:00Z",
          running,
          agentOf("3f2a", "n-laptop"),
        ),
      ],
      "build-server": [
        session(
          "c07e",
          "2026-11-07T10:02:00Z",
          exited,
          agentOf("a91c", "n-gpu"),
        ),
      ],
    },
  };
}

describe("the session tree", () => {
  it("nests agent-started sessions under their creator, across nodes", () => {
    const rows = tree(state(running), ["laptop", "gpu-box", "build-server"]);
    expect(rows.map((row) => [row.session.id, row.depth, row.node])).toEqual([
      ["9aa0", 0, "laptop"],
      ["3f2a", 0, "laptop"],
      ["a91c", 1, "gpu-box"],
      ["c07e", 2, "build-server"],
    ]);
    expect(rows[2]?.parent?.id).toBe("3f2a");
    // A child's parent may run on another node; the row says which.
    expect(rows.map((row) => row.parentNode)).toEqual([
      null,
      null,
      "laptop",
      "gpu-box",
    ]);
  });

  it("makes a session whose creator is not listed a root", () => {
    const rows = tree(state(running), ["gpu-box", "build-server"]);
    expect(rows.map((row) => [row.session.id, row.depth])).toEqual([
      ["a91c", 0],
      ["c07e", 1],
    ]);
  });
});
