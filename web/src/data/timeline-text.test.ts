import { describe, expect, it } from "vitest";

import { spoken } from "./timeline-text";

const base = { seq: 1, at: "2026-11-11T00:00:00Z" };

describe("an entry as a line", () => {
  it("says what the row shows, without its markup", () => {
    expect(
      spoken({ ...base, key: "a", kind: "message", text: "**Done**" }),
    ).toBe("**Done**");
    expect(
      spoken({
        ...base,
        key: "b",
        kind: "tool",
        tool: {
          id: "call-1",
          title: "Read status.rs",
          kind: "read",
          status: "in_progress",
          content: [],
          locations: [],
          input: null,
          output: null,
        },
      }),
    ).toBe("Read status.rs · in progress");
    expect(
      spoken({ ...base, key: "c", kind: "turn_ended", reason: "end_turn" }),
    ).toBe("Turn ended · end turn");
    expect(
      spoken({
        ...base,
        key: "d",
        kind: "plan",
        entries: [
          { content: "a", priority: "high", status: "completed" },
          { content: "b", priority: "low", status: "pending" },
        ],
      }),
    ).toBe("Plan · 1 of 2 done");
  });
});
