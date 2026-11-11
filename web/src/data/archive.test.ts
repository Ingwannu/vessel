import { describe, expect, it } from "vitest";

import type { SessionInfo } from "../wire/proto";
import { searchArchive } from "./archive";
import { type Card, placement } from "./board";
import type { FleetState } from "./fleet";

function card(node: string, id: string, cwd: string): Card {
  const session: SessionInfo = {
    id,
    spec: { harness: "claude-code", mode: "structured", cwd },
    created_by: { kind: "owner" },
    status: { state: "exited", code: 0 },
    created_at: "2026-11-07T10:00:00Z",
    next_seq: 0,
  };
  return {
    node,
    session,
    depth: 0,
    parent: null,
    parentNode: null,
    placed: placement(session),
  };
}

const state: FleetState = {
  link: { kind: "open", version: "0.1.0" },
  book: [
    {
      key: "gpu",
      label: "gpu-box",
      address: "x",
      transport: "ws",
      default: false,
      id: null,
    },
  ],
  bookError: null,
  statuses: {},
  harnesses: {},
  profiles: {},
  models: {},
  sessions: {},
  errors: {},
  selected: null,
};

const cards = [
  card("gpu", "3f2a99", "/w/Vessel"),
  card("laptop", "9aa000", "/w/docs"),
];

describe("searchArchive", () => {
  it("keeps everything for an empty query", () => {
    expect(searchArchive(cards, state, "  ")).toBe(cards);
  });

  it("matches an id's start, a folder, or a node's name, in any case", () => {
    const ids = (query: string) =>
      searchArchive(cards, state, query).map((found) => found.session.id);
    expect(ids("3f2a")).toEqual(["3f2a99"]);
    expect(ids("2a99")).toEqual([]);
    expect(ids("vessel")).toEqual(["3f2a99"]);
    expect(ids("GPU-BOX")).toEqual(["3f2a99"]);
    expect(ids("laptop")).toEqual(["9aa000"]);
  });
});
