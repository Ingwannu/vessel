import { describe, expect, it } from "vitest";

import { fixtureId } from "../../testing/ids";
import type {
  SourceReportEntry,
  UsageReports,
  UsageSourceDescriptor,
} from "../../wire/proto";
import { QuotaInstant } from "./clock";
import { balanceAmount, quotaCards } from "./cards";
import { consentRows } from "./consent";

const NOW = QuotaInstant.fromIso("2026-11-16T09:00:00Z");
if (NOW === null) throw new Error("fixture clock");

function source(
  id: string,
  availability: UsageSourceDescriptor["availability"],
  consent: UsageSourceDescriptor["consent"] = { kind: "inherited_global" },
): UsageSourceDescriptor {
  return {
    id: fixtureId("UsageSourceId", id),
    label: id,
    builtin: null,
    disclosure: { version: 2, store: "cursor", recipient: "cursor" },
    consent,
    availability,
  };
}

function entry(
  id: string,
  state: SourceReportEntry["state"],
  sourceId = "codex",
): SourceReportEntry {
  return {
    id: fixtureId("UsageReportId", id),
    source: fixtureId("UsageSourceId", sourceId),
    account: fixtureId("UsageAccountKey", `${id}-account`),
    source_label: "Codex saved accounts",
    builtin: "codex",
    saved_login: "codex",
    account_label: "codex",
    state,
  };
}

const observed = {
  at: "2026-11-16T08:50:00Z",
  report: {
    completeness: "complete" as const,
    plan: "Plus",
    windows: [
      {
        id: "general:weekly",
        label: "Weekly window",
        short_label: null,
        summary: "unspecified" as const,
        used: 96,
        remaining: null,
        resets_at: "2026-11-18T07:53:00Z",
        runs_out_at: null,
        shortfall: null,
        tone: null,
      },
    ],
    balances: [
      {
        id: "credits",
        label: "Reported credits",
        used: 0.30000000000000004,
        remaining: null,
        limit: 10,
        unit: "credits" as const,
        resets_at: null,
        tone: null,
      },
    ],
    details: [
      {
        id: "plan_note",
        label: "Visible",
        value: "yes",
        access: "read" as const,
        tone: null,
      },
      {
        id: "secret",
        label: "Operate only",
        value: "hidden",
        access: "operate" as const,
        tone: null,
      },
    ],
  },
};

function reports(
  list: SourceReportEntry[],
  sources: UsageSourceDescriptor[] = [source("codex", "enabled")],
  visibility: UsageReports["cost_visibility"] = "withheld",
): UsageReports {
  return {
    at: "2026-11-16T09:00:00Z",
    policy: "on",
    cost_visibility: visibility,
    sources,
    reports: list,
    source_failures: [],
  };
}

describe("quotaCards", () => {
  it("draws a current report with Paseo's states and hides operate-only facts", () => {
    const [card] = quotaCards(
      reports([entry("a", { state: "available", observed })]),
      "used",
      "en",
      NOW,
    );
    expect(card?.status).toBeNull();
    expect(card?.plan).toBe("Plus");
    expect(card?.windows[0]?.percent?.percent).toBe(96);
    expect(card?.windows[0]?.tone).toBe("danger");
    expect(card?.windows[0]?.clock.kind).toBe("reset");
    expect(card?.details.map((detail) => detail.label)).toEqual(["Visible"]);
    expect(card?.refreshable).toBe(true);
    // Withheld visibility hides balance amounts as well.
    expect(card?.balances[0]?.amount).toBe("Hidden for this device");
  });

  it("keeps a retained success beside a failed attempt's error", () => {
    const [card] = quotaCards(
      reports([
        entry("a", {
          state: "failed",
          attempted_at: null,
          reason: "timed_out",
          retry_at: null,
          retained: observed,
          logins: [],
        }),
      ]),
      "remaining",
      "en",
      NOW,
    );
    expect(card?.status).toBe("error");
    expect(card?.notes).toEqual(["The usage request timed out. Try again."]);
    expect(card?.windows[0]?.percent?.percent).toBe(4);
    expect(card?.fetchedAt?.milliseconds).toBe(Date.parse(observed.at));
  });

  it("reads an unsupported source as Unavailable and never refreshable", () => {
    const [card] = quotaCards(
      reports(
        [entry("a", { state: "not_requested" }, "gemini")],
        [source("gemini", "unsupported")],
      ),
      "used",
      "en",
      NOW,
    );
    expect(card?.status).toBe("unavailable");
    expect(card?.notChecked).toBe(true);
    expect(card?.refreshable).toBe(false);
  });
});

describe("balanceAmount", () => {
  const credits = observed.report.balances[0];
  if (credits === undefined) throw new Error("fixture balance");

  it("writes Paseo's used / limit with the binary tail rounded away", () => {
    expect(balanceAmount(credits, "permitted", "en")).toMatchObject({
      amount: "0.3 / 10",
    });
    expect(
      balanceAmount(
        { ...credits, used: 1234.5, limit: null, unit: "usd" },
        "permitted",
        "en",
      ).amount,
    ).toBe("$1,234.50");
    expect(
      balanceAmount(
        { ...credits, used: null, remaining: 7, limit: null },
        "permitted",
        "ko",
      ).amount,
    ).toBe("7 left");
  });
});

describe("consentRows", () => {
  it("offers a choice only for sources with their own consent", () => {
    const rows = consentRows(
      reports(
        [],
        [
          source("codex", "enabled"),
          source("gemini", "unsupported", { kind: "off" }),
          source("cursor", "enabled", { kind: "granted", version: 2 }),
          source("grok", "disclosure_needs_consent", {
            kind: "granted",
            version: 1,
          }),
        ],
      ),
    );
    expect(rows.map((row) => [row.source.id, row.next])).toEqual([
      ["cursor", "off"],
      ["grok", "on"],
    ]);
    expect(rows[0]?.disclosure).toContain("send that sign-in to Cursor");
  });
});
