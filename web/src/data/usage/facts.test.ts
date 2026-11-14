import { describe, expect, it } from "vitest";

import { usageReport } from "../fixtures/usage";
import { usageCount, usageFacts, usageTime } from "./facts";

describe("reported accounting facts", () => {
  it("preserves reported zero, independent cache fields and exact cost units", () => {
    expect(usageFacts(usageReport(), "en")).toEqual([
      { label: "Total tokens", value: "30", detail: "1 of 1 turns reported" },
      { label: "Input tokens", value: "20", detail: "1 of 1 turns reported" },
      { label: "Output tokens", value: "10", detail: "1 of 1 turns reported" },
      {
        label: "Reasoning tokens",
        value: "0",
        detail: "1 of 1 turns reported",
      },
      {
        label: "Cache read tokens",
        value: "12",
        detail: "1 of 1 turns reported",
      },
      {
        label: "Cache write tokens",
        value: "0",
        detail: "1 of 1 turns reported",
      },
      { label: "Turns without token reports", value: "0", detail: null },
      {
        label: "Reported cost (USD)",
        value: "0.5",
        detail:
          "Only reported increments attributed to this interval; no price estimates.",
      },
    ]);
  });

  it("does not turn missing reports or scope redaction into zero", () => {
    const report = usageReport();
    report.totals.tokens = null;
    report.totals.reported = 0;
    report.totals.unreported = 3;
    report.totals.optional_reports = {
      thought: 0,
      cached_read: 0,
      cached_write: 0,
    };
    report.cost_visibility = "withheld";
    expect(usageFacts(report, "en")).toEqual([
      ...[
        "Total tokens",
        "Input tokens",
        "Output tokens",
        "Reasoning tokens",
        "Cache read tokens",
        "Cache write tokens",
      ].map((label) => ({
        label,
        value: "Not reported",
        detail: "0 of 3 turns reported",
      })),
      { label: "Turns without token reports", value: "3", detail: null },
      {
        label: "Reported cost",
        value: "Hidden for this device",
        detail: "Operate access is needed to see cost.",
      },
    ]);
  });

  it("keeps per-field coverage and all unreported turns alongside partial sums", () => {
    const report = usageReport();
    report.totals.reported = 2;
    report.totals.unreported = 1;
    report.totals.optional_reports = {
      thought: 1,
      cached_read: 2,
      cached_write: 0,
    };
    report.totals.tokens = {
      total: 30,
      input: 20,
      output: 10,
      thought: 0,
      cached_read: 12,
    };
    report.totals.costs = [];
    expect(usageFacts(report, "en")).toEqual([
      { label: "Total tokens", value: "30", detail: "2 of 3 turns reported" },
      { label: "Input tokens", value: "20", detail: "2 of 3 turns reported" },
      { label: "Output tokens", value: "10", detail: "2 of 3 turns reported" },
      {
        label: "Reasoning tokens",
        value: "0",
        detail: "1 of 3 turns reported",
      },
      {
        label: "Cache read tokens",
        value: "12",
        detail: "2 of 3 turns reported",
      },
      {
        label: "Cache write tokens",
        value: "Not reported",
        detail: "0 of 3 turns reported",
      },
      { label: "Turns without token reports", value: "1", detail: null },
      { label: "Reported cost", value: "Not reported", detail: null },
    ]);
  });

  it("retains separate units, tiny exact reported values and omitted-cost coverage", () => {
    const report = usageReport();
    report.totals.costs = [
      { amount: 0, unit: "USD" },
      { amount: 1e-28, unit: "credits" },
    ];
    report.totals.unaccounted_costs = 2;
    report.totals.token_accounting = "overflow";
    report.incomplete = true;
    expect(usageFacts(report, "en").slice(-4)).toEqual([
      {
        label: "Reported cost (USD)",
        value: "0",
        detail:
          "Only reported increments attributed to this interval; no price estimates.",
      },
      {
        label: "Reported cost (credits)",
        value: "0.0000000000000000000000000001",
        detail:
          "Only reported increments attributed to this interval; no price estimates.",
      },
      {
        label: "Unattributed cost reports",
        value: "2",
        detail:
          "Unknown timing or numerical precision prevents assigning these reports to this interval.",
      },
      {
        label: "Coverage",
        value: "Partial report",
        detail: "Some token totals exceed the supported counter range.",
      },
    ]);
    report.totals.token_accounting = "reported";
    expect(usageFacts(report, "en").at(-1)).toEqual({
      label: "Coverage",
      value: "Partial report",
      detail:
        "Retention or grouping limits omitted data; totals cover retained samples.",
    });
  });

  it("does not print rounded coverage counters as exact turn counts", () => {
    const report = usageReport();
    report.totals.reported = Number.MAX_SAFE_INTEGER;
    report.totals.unreported = 1;
    report.totals.unaccounted_costs = Number("18446744073709551615");
    const facts = usageFacts(report, "en");
    expect(facts[0]?.detail).toBe(
      "Turn coverage is not exactly representable in this browser.",
    );
    expect(facts.at(-1)?.value).toBe(
      "Not exactly representable in this browser",
    );
  });

  it("formats quantities in the viewer locale without pretending unsafe integers are exact", () => {
    expect(usageCount(1_200_000, "en")).toBe("1.2M");
    expect(usageCount(1_200_000, "ko")).toBe("120만");
    expect(usageCount(0, "en")).toBe("0");
    expect(usageCount(null, "en")).toBe("Not reported");
    expect(usageCount(Number("18446744073709551615"), "en")).toBe(
      "Not exactly representable in this browser",
    );
    expect(usageTime("not a timestamp", "en")).toBe("Not available");
  });
});
