import { describe, expect, it } from "vitest";

import { composerPlaceholder } from "./composer-words";

describe("the composer's placeholder", () => {
  it("says a message is queued behind a first prompt while the agent starts", () => {
    expect(
      composerPlaceholder({
        pending: false,
        live: true,
        again: false,
        plan: { kind: "stop" },
        modern: true,
        starting: "Claude Code",
      }),
    ).toBe("Queue a follow-up while Claude Code gets ready");
  });

  it("says what it said before once the agent has the prompt", () => {
    expect(
      composerPlaceholder({
        pending: false,
        live: true,
        again: false,
        plan: { kind: "idle" },
        modern: true,
        starting: null,
      }),
    ).toBe("Message the agent, tag @files, or use /commands");
  });

  it("asks for a pending permission first", () => {
    expect(
      composerPlaceholder({
        pending: true,
        live: true,
        again: false,
        plan: { kind: "idle" },
        modern: true,
        starting: "Claude Code",
      }),
    ).toBe("Answer the permission to continue");
  });
});
