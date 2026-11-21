import { describe, expect, it } from "vitest";

import type { ForgeAccess } from "../../../wire/proto";
import { unavailableOf, type Unavailable } from "./unavailable";

describe("why there is no pull request (entry.rs's Unavailable::of)", () => {
  it("keeps each forge refusal's host and time", () => {
    const host = "github.com";
    const at = "2026-11-14T10:00:00Z";
    const cases: [ForgeAccess, Unavailable][] = [
      [{ access: "no_branch" }, { kind: "noBranch" }],
      [{ access: "no_remote" }, { kind: "noRemote" }],
      [
        { access: "not_supported", host },
        { kind: "notSupported", host },
      ],
      [
        { access: "signed_out", host, gh: "missing" },
        { kind: "signedOut", host },
      ],
      [
        { access: "refused", host, reason: { kind: "unauthorized" } },
        { kind: "refused", host },
      ],
      [
        { access: "rate_limited", host, until: at },
        { kind: "rateLimited", host, until: at },
      ],
      [
        { access: "unreachable", host, retry_at: at },
        { kind: "unreachable", host, retryAt: at },
      ],
      [{ access: "limit" }, { kind: "limit" }],
      [{ access: "ended" }, { kind: "ended" }],
      [{ access: "unknown" }, { kind: "unknown" }],
    ];
    for (const [access, why] of cases) {
      expect(unavailableOf(access)).toEqual(why);
    }
  });

  it("reads an access that is no refusal as unknown, as the native rule", () => {
    const fine: ForgeAccess[] = [
      { access: "ready" },
      { access: "on_base" },
      { access: "not_placed" },
    ];
    for (const access of fine) {
      expect(unavailableOf(access)).toEqual({ kind: "unknown" });
    }
  });
});
