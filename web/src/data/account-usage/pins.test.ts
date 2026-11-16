import { describe, expect, it } from "vitest";

import { fixtureId } from "../../testing/ids";
import type {
  SourceReportEntry,
  UsageReports,
  UsageWindow,
} from "../../wire/proto";
import { effectivePins, togglePin } from "./pins";

function window(id: string, summary: UsageWindow["summary"]): UsageWindow {
  return {
    id,
    label: id,
    short_label: null,
    summary,
    used: 40,
    remaining: null,
    resets_at: null,
    runs_out_at: null,
    shortfall: null,
    tone: null,
  };
}

function entry(source: string, windows: UsageWindow[]): SourceReportEntry {
  return {
    id: fixtureId("UsageReportId", `${source}:a`),
    source: fixtureId("UsageSourceId", source),
    account: fixtureId("UsageAccountKey", "a"),
    source_label: source,
    builtin: null,
    saved_login: null,
    account_label: null,
    state: {
      state: "available",
      observed: {
        at: "2026-11-16T09:00:00Z",
        report: {
          completeness: "complete",
          plan: null,
          windows,
          balances: [],
          details: [],
        },
      },
    },
  };
}

const REPORTS: UsageReports = {
  at: "2026-11-16T09:00:00Z",
  policy: "on",
  cost_visibility: "permitted",
  sources: [],
  reports: [
    entry("codex", [
      window("five_hour", "unspecified"),
      window("weekly", "unspecified"),
    ]),
    entry("cursor", [
      window("monthly", "unspecified"),
      window("daily", "included"),
    ]),
  ],
  source_failures: [],
};

describe("pins", () => {
  it("defaults to marked windows, else each source's first with a percent", () => {
    expect(effectivePins(null, REPORTS, "used")).toEqual([
      { source: "codex", window: "five_hour" },
      { source: "cursor", window: "daily" },
    ]);
    // An explicit empty list pins nothing.
    expect(effectivePins([], REPORTS, "used")).toEqual([]);
  });

  it("keeps the shown defaults on the first edit, then toggles one", () => {
    const pin = {
      source: fixtureId("UsageSourceId", "codex"),
      window: "weekly",
    };
    expect(togglePin(null, pin, REPORTS, "used")).toEqual([
      { source: "codex", window: "five_hour" },
      { source: "cursor", window: "daily" },
      { source: "codex", window: "weekly" },
    ]);
    const off = {
      source: fixtureId("UsageSourceId", "cursor"),
      window: "daily",
    };
    expect(togglePin(null, off, REPORTS, "used")).toEqual([
      { source: "codex", window: "five_hour" },
    ]);
  });

  it("has no windows while lookup is off", () => {
    expect(effectivePins(null, { ...REPORTS, policy: "off" }, "used")).toEqual(
      [],
    );
  });
});
