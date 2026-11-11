// Machines, statuses and agents for the tests of Settings' data modules
// (track W2). Generic names; nothing here is a real machine.

import type { BookNode, NodeStatus } from "../../wire/bridge";
import type { Harness, SessionInfo } from "../../wire/proto";
import { fixtureId } from "../../testing/ids";

export function bookNode(key: string, extra: Partial<BookNode> = {}): BookNode {
  return {
    key: fixtureId("NodeKey", key),
    label: key,
    address: "ws://127.0.0.1:7002/rpc",
    transport: "ws",
    default: false,
    id: null,
    ...extra,
  };
}

export function connectedAs(name: string, os = "linux"): NodeStatus {
  return {
    state: "connected",
    protocol: 9,
    node: {
      id: fixtureId("NodeId", `id-${name}`),
      name,
      hostname: name,
      os,
      arch: "x86_64",
      version: "0.1.0",
      protocol: { min: 2, max: 9 },
      started_at: "2026-11-11T06:00:00Z",
      state: "serving",
    },
  };
}

export const claude: Harness = {
  kind: "claude-code",
  path: "/usr/local/bin/claude",
  version: "2.1.0",
  structured: { state: "available" },
};

export const codexTerminalOnly: Harness = {
  kind: "codex",
  path: "/usr/local/bin/codex",
  version: "0.50.0",
  structured: { state: "unavailable", reason: "codex-acp is not installed" },
};

export const liveSession: SessionInfo = {
  id: fixtureId("SessionId", "aaaa"),
  spec: { harness: "claude-code", mode: "structured" },
  created_by: { kind: "owner" },
  status: { state: "running" },
  created_at: "2026-11-11T06:01:00Z",
  next_seq: 0,
};
