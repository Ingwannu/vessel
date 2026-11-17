import { describe, expect, it } from "vitest";

import type { LimitNotice, UsageLimits } from "../../wire/proto";
import { QuotaInstant } from "./clock";
import { sessionNotice } from "./session-notice";

const NOW = QuotaInstant.fromIso("2026-11-17T12:00:00Z");

function limits(notice: LimitNotice | null): UsageLimits {
  return {
    policy: "on",
    account: { harness: "codex", profile: null },
    source: null,
    state: { state: "not_requested" },
    cost_visibility: "permitted",
    notice,
  };
}

describe("a session's limit notice", () => {
  it("is absent unless the node reports one", () => {
    if (NOW === null) throw new Error("Fixture clock must parse");
    expect(sessionNotice(limits(null), NOW)).toBeNull();
  });

  it("keeps the reported reset only while it is ahead", () => {
    if (NOW === null) throw new Error("Fixture clock must parse");
    const ahead = sessionNotice(
      limits({
        reported_full: ["five_hour"],
        latest_reported_reset: "2026-11-17T14:00:00Z",
      }),
      NOW,
    );
    expect(ahead?.reset?.milliseconds).toBe(Date.parse("2026-11-17T14:00:00Z"));
    const passed = sessionNotice(
      limits({
        reported_full: ["five_hour"],
        latest_reported_reset: "2026-11-17T11:00:00Z",
      }),
      NOW,
    );
    expect(passed).toEqual({ reset: null });
    const unknown = sessionNotice(
      limits({ reported_full: ["weekly"], latest_reported_reset: null }),
      NOW,
    );
    expect(unknown).toEqual({ reset: null });
  });
});
