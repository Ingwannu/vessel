import { describe, expect, it } from "vitest";

import type { BookNode, NodeStatus } from "../wire/bridge";
import type { SessionInfo } from "../wire/proto";
import { nodeFacts, reach } from "./nodes";

const box: BookNode = {
  key: "gpu-box",
  label: "gpu-box",
  address: "ws://127.0.0.1:7002/rpc",
  transport: "ws",
  default: false,
  id: null,
};

const connected: NodeStatus = {
  state: "connected",
  protocol: 9,
  node: {
    id: "n-1",
    name: "gpu-box",
    hostname: "gpu-box",
    os: "linux",
    arch: "x86_64",
    version: "0.1.0",
    protocol: { min: 2, max: 9 },
    started_at: "2026-11-11T06:00:00Z",
    state: "serving",
  },
};

const live: SessionInfo = {
  id: "aaaa",
  spec: { harness: "claude-code", mode: "structured" },
  created_by: { kind: "owner" },
  status: { state: "running" },
  created_at: "2026-11-11T06:01:00Z",
  next_seq: 0,
};

describe("node facts", () => {
  it("names the route in the page's words", () => {
    expect(reach(box)).toBe("direct");
    expect(reach({ ...box, transport: "paired" })).toBe("relay");
  });

  it("says where an unconnected node is and why it is not here", () => {
    expect(
      nodeFacts(
        box,
        { state: "failed", message: "refused" },
        undefined,
        undefined,
      ),
    ).toEqual(["ws://127.0.0.1:7002/rpc", "direct", "offline"]);
  });

  it("says what a connected node runs and how busy it is", () => {
    expect(
      nodeFacts(
        box,
        connected,
        [
          {
            kind: "claude-code",
            path: "/bin/claude",
            version: null,
            structured: { state: "available" },
          },
        ],
        [live, { ...live, id: "bbbb", status: { state: "exited", code: 0 } }],
      ),
    ).toEqual(["linux/x86_64", "direct", "claude", "1 live", "0.1.0"]);
  });
});
