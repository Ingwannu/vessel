import { describe, expect, it } from "vitest";

import { CallFailed } from "../bridge/client";
import { OpFailed } from "../bridge/frames";
import { attempt, nextStep } from "./outcome";

describe("an outcome", () => {
  it("keeps the bridge's reason for an op and a call", async () => {
    const op = await attempt(() =>
      Promise.reject(new OpFailed("busy", "too many calls")),
    );
    expect(op).toEqual({
      ok: false,
      problem: { from: "op", reason: "busy", message: "too many calls" },
    });
    const call = await attempt(() =>
      Promise.reject(new CallFailed({ kind: "disconnected" })),
    );
    expect(call.ok ? null : call.problem.from).toBe("call");
    expect(await attempt(() => Promise.resolve(3))).toEqual({
      ok: true,
      value: 3,
    });
  });

  it("says what to do next by the reason, not the message", () => {
    expect(
      nextStep({ from: "op", reason: "busy", message: "anything" }),
    ).toMatch(/try again in a moment/);
    expect(
      nextStep({
        from: "call",
        failure: {
          kind: "not_in_protocol",
          method: "harness.options",
          needs: 9,
          negotiated: 8,
        },
        message: "",
      }),
    ).toMatch(/older Vessel/);
  });
});
