import { describe, expect, it } from "vitest";

import type { SharedPull } from "./diff-stat";
import {
  badgeOf,
  badgeWord,
  checkSummary,
  metaItems,
  workspaceName,
} from "./meta-row";

function pull(over: Partial<SharedPull> = {}): SharedPull {
  return {
    kind: "github",
    host: "github.com",
    path: "acme-labs/fleet-agent",
    number: 128,
    state: "open",
    draft: false,
    checks: "pending",
    counts: { passed: 4, failed: 0, pending: 2 },
    url: "https://github.com/acme-labs/fleet-agent/pull/128",
    ...over,
  };
}

describe("the workspace line (TRACKER 426)", () => {
  it("fills the running pie by quarters, rounded down", () => {
    const quarters = (passed: number, failed: number, pending: number) =>
      checkSummary(pull({ counts: { passed, failed, pending } }))?.quarters;
    expect(quarters(0, 0, 4)).toBe(0);
    expect(quarters(2, 0, 4)).toBe(1);
    expect(quarters(4, 0, 2)).toBe(2);
    expect(quarters(5, 0, 1)).toBe(3);
    expect(quarters(9, 0, 1)).toBe(3);
    expect(quarters(3, 0, 0)).toBe(4);
  });

  it("keeps the ring empty without a tally, and lets a failure win", () => {
    expect(checkSummary(pull({ counts: null }))).toEqual({
      outcome: "running",
      quarters: 0,
    });
    expect(checkSummary(pull({ checks: "failing" }))?.outcome).toBe("failed");
    expect(checkSummary(pull({ checks: "cancelled" }))?.outcome).toBe("failed");
    expect(checkSummary(pull({ checks: "passing" }))?.outcome).toBe("passed");
    // A summary still pending whose tally already holds a failure.
    expect(
      checkSummary(pull({ counts: { passed: 2, failed: 1, pending: 3 } }))
        ?.outcome,
    ).toBe("failed");
    expect(checkSummary(pull({ checks: "none" }))).toBeNull();
    expect(checkSummary(null)).toBeNull();
  });

  it("says a state only when the pull request is not open", () => {
    expect(badgeWord(badgeOf(pull()))).toBeNull();
    expect(badgeWord(badgeOf(pull({ draft: true })))).toBeNull();
    expect(badgeWord(badgeOf(pull({ state: "merged" })))).toBe("merged");
    expect(badgeWord(badgeOf(pull({ state: "closed" })))).toBe("closed");
    expect(badgeWord(badgeOf(pull({ state: "unknown" })))).toBeNull();
  });

  it("orders the items as Paseo's: machine, pull request, checks, labels", () => {
    const kinds = metaItems({
      machine: "workstation",
      pull: pull(),
      labels: 2,
    }).map((item) => item.kind);
    expect(kinds).toEqual(["host", "changeRequest", "checks", "labels"]);
    expect(metaItems({ machine: null, pull: null, labels: 0 })).toEqual([]);
    const quiet = metaItems({
      machine: null,
      pull: pull({ checks: "none" }),
      labels: 0,
    });
    expect(quiet.map((item) => item.kind)).toEqual(["changeRequest"]);
  });

  it("names the row with its title, machine and pull request", () => {
    expect(
      workspaceName({
        title: "fleet-agent",
        machine: "workstation",
        pull: pull(),
      }),
    ).toBe("fleet-agent, workstation, Pull request #128");
    expect(
      workspaceName({ title: "fleet-agent", machine: null, pull: null }),
    ).toBe("fleet-agent");
  });
});
