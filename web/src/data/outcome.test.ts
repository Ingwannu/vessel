import { describe, expect, it } from "vitest";

import { CallFailed } from "../bridge/client";
import { OpFailed } from "../bridge/frames";
import { attempt, nextStep, withNextStep } from "./outcome";

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

  it("joins the message and the next step as two sentences (TRACKER 820)", () => {
    const failed = (message: string) => ({
      from: "op" as const,
      reason: "failed" as const,
      message,
    });
    // A node's message with no stop gets one; its own words stay.
    expect(
      withNextStep(
        failed(
          "this device may not call `device.list` (it needs the `admin` scope)",
        ),
      ),
    ).toBe(
      "this device may not call `device.list` (it needs the `admin` scope). Nothing was changed.",
    );
    // A message that already ends a sentence is left as it is.
    expect(withNextStep(failed("The book is locked."))).toBe(
      "The book is locked. Nothing was changed.",
    );
    expect(withNextStep(failed("Gone!"))).toBe("Gone! Nothing was changed.");
    expect(withNextStep(failed("Still there?"))).toBe(
      "Still there? Nothing was changed.",
    );
    // Trailing space is not a stop; an empty message leaves the next step.
    expect(withNextStep(failed("no route  "))).toBe(
      "no route. Nothing was changed.",
    );
    expect(withNextStep(failed(""))).toBe("Nothing was changed.");
  });
});
