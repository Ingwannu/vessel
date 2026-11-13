import { describe, expect, it } from "vitest";

import type { SessionInfo } from "../wire/proto";
import type { FleetState } from "./fleet";
import { keep, withoutSession, withSession } from "./fleet-sessions";
import { fixtureId } from "../testing/ids";

const LAPTOP = fixtureId("NodeKey", "laptop");
const OTHER = fixtureId("NodeKey", "gpu-box");

function session(id: string, title?: string): SessionInfo {
  return {
    id: fixtureId("SessionId", id),
    spec: { harness: "claude-code", mode: "structured" },
    created_by: { kind: "owner" },
    status: { state: "running" },
    created_at: "2026-11-13T00:00:00Z",
    next_seq: 0,
    ...(title === undefined ? {} : { title }),
  };
}

function state(sessions: FleetState["sessions"]): FleetState {
  return {
    link: { kind: "open", version: "0.1.0" },
    book: null,
    bookError: null,
    statuses: {},
    harnesses: {},
    profiles: {},
    models: {},
    agents: {},
    sessions,
    errors: {},
    selected: null,
  };
}

describe("a machine's sessions", () => {
  it("keep their objects where a list read again says the same", () => {
    const before = [session("a"), session("b")];
    expect(keep(before, [session("a"), session("b")])).toBe(before);
    const after = keep(before, [session("a"), session("b", "renamed")]);
    expect(after[0]).toBe(before[0]);
    expect(after[1]).not.toBe(before[1]);
  });

  it("take a session the node answered with, in place or first", () => {
    const listed = state({ [LAPTOP]: [session("a"), session("b")] });
    const renamed = session("b", "renamed");
    expect(withSession(listed, LAPTOP, renamed).sessions[LAPTOP]).toEqual([
      session("a"),
      renamed,
    ]);
    expect(
      withSession(listed, LAPTOP, session("c")).sessions[LAPTOP]?.[0]?.id,
    ).toBe("c");
    // A machine never read is left alone.
    expect(withSession(listed, OTHER, session("c"))).toBe(listed);
  });

  it("let a deleted session go", () => {
    const listed = state({ [LAPTOP]: [session("a"), session("b")] });
    expect(
      withoutSession(listed, LAPTOP, fixtureId("SessionId", "a")).sessions[
        LAPTOP
      ],
    ).toEqual([session("b")]);
    expect(withoutSession(listed, OTHER, fixtureId("SessionId", "a"))).toBe(
      listed,
    );
  });
});
