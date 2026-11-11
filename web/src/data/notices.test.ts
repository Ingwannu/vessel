import { describe, expect, test } from "vitest";

import type { DisplayStatus, SessionInfo } from "../wire/proto";
import { notices, noticeTitle, titleWith } from "./notices";
import { fixtureId } from "../testing/ids";

function session(
  id: string,
  status: DisplayStatus | null,
  live = true,
): SessionInfo {
  return {
    id: fixtureId("SessionId", id),
    spec: { harness: "claude-code", mode: "structured", cwd: `/work/${id}` },
    created_by: { kind: "owner" },
    status: live ? { state: "running" } : { state: "exited", code: 0 },
    created_at: "2026-11-11T06:00:00Z",
    next_seq: 0,
    ...(status === null
      ? {}
      : {
          board: {
            column: "building",
            display_status: status,
            activity: "active",
            last_activity_at: "2026-11-11T06:00:00Z",
          },
        }),
  };
}

describe("what is worth telling", () => {
  test("a session starts to need you", () => {
    const found = notices(
      fixtureId("NodeKey", "lab"),
      [session("a", "working")],
      [session("a", "blocked")],
    );
    expect(found.map((notice) => notice.kind)).toEqual(["needs-you"]);
  });

  test("a working session goes idle or exits: finished", () => {
    expect(
      notices(
        fixtureId("NodeKey", "lab"),
        [session("a", "working")],
        [session("a", "idle")],
      )[0]?.kind,
    ).toBe("finished");
    expect(
      notices(
        fixtureId("NodeKey", "lab"),
        [session("a", "working")],
        [session("a", "exited", false)],
      )[0]?.kind,
    ).toBe("finished");
  });

  test("nothing new: still waiting, still working, idle again, a first sighting", () => {
    expect(
      notices(
        fixtureId("NodeKey", "lab"),
        [session("a", "blocked")],
        [session("a", "blocked")],
      ),
    ).toEqual([]);
    expect(
      notices(
        fixtureId("NodeKey", "lab"),
        [session("a", "working")],
        [session("a", "working")],
      ),
    ).toEqual([]);
    expect(
      notices(
        fixtureId("NodeKey", "lab"),
        [session("a", "idle")],
        [session("a", "idle")],
      ),
    ).toEqual([]);
    expect(
      notices(fixtureId("NodeKey", "lab"), [], [session("b", "blocked")]),
    ).toEqual([]);
  });

  test("an answered permission that goes back to work says nothing", () => {
    expect(
      notices(
        fixtureId("NodeKey", "lab"),
        [session("a", "blocked")],
        [session("a", "working")],
      ),
    ).toEqual([]);
  });

  test("an older node's session finishes when it ends", () => {
    expect(
      notices(
        fixtureId("NodeKey", "lab"),
        [session("a", null)],
        [session("a", null, false)],
      )[0]?.kind,
    ).toBe("finished");
  });
});

test("titles", () => {
  expect(noticeTitle("needs-you")).toBe("Session needs you");
  expect(titleWith("Vessel", 2)).toBe("(2) Vessel");
  expect(titleWith("Vessel", 0)).toBe("Vessel");
});
