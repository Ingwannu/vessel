import { describe, expect, it } from "vitest";

import type { PermissionOption } from "../wire/proto";
import { answersOf, settled, settledLine } from "./approval";

const ALL: PermissionOption[] = [
  { id: "always", name: "Always allow", kind: "allow_always" },
  { id: "allow", name: "Allow", kind: "allow_once" },
  { id: "reject", name: "Reject", kind: "reject_once" },
  { id: "never", name: "Never", kind: "reject_always" },
];

describe("answersOf", () => {
  it("puts the once-options on the buttons and the rest under More", () => {
    const answers = answersOf(ALL);
    expect(answers.approve?.name).toBe("Allow");
    expect(answers.decline?.name).toBe("Reject");
    expect(answers.more.map((option) => option.name)).toEqual([
      "Always allow",
      "Never",
    ]);
  });

  it("falls back to any allow or reject, and to none", () => {
    const answers = answersOf([
      { id: "always", name: "Always allow", kind: "allow_always" },
      { id: "never", name: "Never", kind: "reject_always" },
    ]);
    expect(answers.approve?.name).toBe("Always allow");
    expect(answers.decline?.name).toBe("Never");
    expect(answers.more).toEqual([]);
    expect(answersOf([])).toEqual({ approve: null, decline: null, more: [] });
  });
});

describe("settled", () => {
  it("says how a request ended, in the agent's words", () => {
    expect(settled(ALL, null)).toEqual({ state: "pending" });
    expect(settled(ALL, { outcome: "selected", option: "allow" })).toEqual({
      state: "allowed",
      option: "Allow",
    });
    expect(settled(ALL, { outcome: "selected", option: "never" })).toEqual({
      state: "declined",
      option: "Never",
    });
    expect(settled(ALL, { outcome: "cancelled" })).toEqual({
      state: "cancelled",
    });
    expect(
      settledLine(
        "Run fake tool",
        settled(ALL, { outcome: "selected", option: "allow" }),
      ),
    ).toBe("Allowed · Run fake tool · Allow");
    expect(settledLine("Run fake tool", { state: "pending" })).toBe(
      "Waiting on permission · Run fake tool",
    );
  });
});
