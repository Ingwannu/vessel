import { describe, expect, it } from "vitest";

import type { PermissionOption } from "../wire/proto";
import { answersOf, settled, settledLine } from "./approval";
import { fixtureId } from "../testing/ids";

const ALL: PermissionOption[] = [
  {
    id: fixtureId("PermissionOptionId", "always"),
    name: "Always allow",
    kind: "allow_always",
  },
  {
    id: fixtureId("PermissionOptionId", "allow"),
    name: "Allow",
    kind: "allow_once",
  },
  {
    id: fixtureId("PermissionOptionId", "reject"),
    name: "Reject",
    kind: "reject_once",
  },
  {
    id: fixtureId("PermissionOptionId", "never"),
    name: "Never",
    kind: "reject_always",
  },
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
      {
        id: fixtureId("PermissionOptionId", "always"),
        name: "Always allow",
        kind: "allow_always",
      },
      {
        id: fixtureId("PermissionOptionId", "never"),
        name: "Never",
        kind: "reject_always",
      },
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
    expect(
      settled(ALL, {
        outcome: "selected",
        option: fixtureId("PermissionOptionId", "allow"),
      }),
    ).toEqual({
      state: "allowed",
      option: "Allow",
    });
    expect(
      settled(ALL, {
        outcome: "selected",
        option: fixtureId("PermissionOptionId", "never"),
      }),
    ).toEqual({
      state: "declined",
      option: "Never",
    });
    expect(settled(ALL, { outcome: "cancelled" })).toEqual({
      state: "cancelled",
    });
    expect(
      settledLine(
        "Run fake tool",
        settled(ALL, {
          outcome: "selected",
          option: fixtureId("PermissionOptionId", "allow"),
        }),
      ),
    ).toBe("Allowed · Run fake tool · Allow");
    expect(settledLine("Run fake tool", { state: "pending" })).toBe(
      "Waiting on permission · Run fake tool",
    );
  });
});
