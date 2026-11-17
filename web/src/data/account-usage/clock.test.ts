import { describe, expect, it } from "vitest";
import {
  QuotaInstant,
  quotaAge,
  quotaClock,
  type QuotaClockInput,
} from "./clock";
import { QuotaPercent } from "./values";

function instant(iso: string): QuotaInstant {
  const value = QuotaInstant.fromIso(iso);
  if (value === null) throw new Error("Invalid test instant");
  return value;
}
const NOW = instant("2026-11-15T12:00:00Z");
const RESET = instant("2026-11-15T14:10:59Z");
const BASE: QuotaClockInput = {
  reset: RESET,
  runOut: null,
  shortfall: null,
  basis: "current",
};

describe("quota time facts", () => {
  it("parses once and keeps invalid or missing times unknown", () => {
    expect(QuotaInstant.fromIso("not a timestamp")).toBeNull();
    expect(QuotaInstant.fromMilliseconds(Number.NaN)).toBeNull();
    expect(QuotaInstant.fromMilliseconds(Number.MAX_SAFE_INTEGER)).toBeNull();
    expect(quotaClock({ ...BASE, reset: null }, NOW)).toEqual({
      kind: "unreported",
    });
  });

  it("floors two whole units and never invents raw second labels", () => {
    expect(quotaClock(BASE, NOW)).toEqual({
      kind: "reset",
      at: RESET,
      distance: "day",
      parts: [
        { unit: "hours", count: 2 },
        { unit: "minutes", count: 10 },
      ],
    });
    const short = instant("2026-11-15T12:00:59Z");
    expect(quotaClock({ ...BASE, reset: short }, NOW)).toEqual({
      kind: "reset",
      at: short,
      distance: "day",
      parts: [{ unit: "minutes", count: 0 }],
    });
    const days = instant("2026-11-18T14:00:00Z");
    expect(quotaClock({ ...BASE, reset: days }, NOW)).toEqual({
      kind: "reset",
      at: days,
      distance: "week",
      parts: [
        { unit: "days", count: 3 },
        { unit: "hours", count: 2 },
      ],
    });
  });

  it("keeps exactly-a-day and exactly-a-week reset boundaries", () => {
    const tomorrow = instant("2026-11-16T12:00:00Z");
    const week = instant("2026-11-22T12:00:00Z");
    expect(quotaClock({ ...BASE, reset: tomorrow }, NOW)).toEqual({
      kind: "reset",
      at: tomorrow,
      distance: "week",
      parts: [{ unit: "days", count: 1 }],
    });
    expect(quotaClock({ ...BASE, reset: week }, NOW)).toEqual({
      kind: "reset",
      at: week,
      distance: "later",
      parts: [{ unit: "days", count: 7 }],
    });
  });

  it("distinguishes an elapsed retained reset from a fresh resetting report", () => {
    expect(quotaClock({ ...BASE, reset: NOW }, NOW)).toEqual({
      kind: "resetting",
    });
    expect(quotaClock({ ...BASE, reset: NOW, basis: "retained" }, NOW)).toEqual(
      {
        kind: "reset_passed",
      },
    );
  });

  it("keeps run-out warnings separate and stops repeating past retained risk", () => {
    const runOut = instant("2026-11-15T12:30:00Z");
    const shortfall = QuotaPercent.from(0);
    const input = { ...BASE, runOut, shortfall };
    expect(quotaClock(input, NOW)).toEqual({
      kind: "risk",
      shortfall,
      parts: [{ unit: "minutes", count: 30 }],
    });
    expect(quotaClock({ ...input, runOut: NOW }, NOW)).toEqual({
      kind: "risk_now",
      shortfall,
    });
    expect(
      quotaClock({ ...input, runOut: NOW, basis: "retained" }, NOW),
    ).toEqual(quotaClock(BASE, NOW));
    expect(quotaClock({ ...input, shortfall: null }, NOW)).toEqual(
      quotaClock(BASE, NOW),
    );
  });

  it("uses successful snapshot age and preserves future clock skew behavior", () => {
    expect(quotaAge(null, NOW)).toBeNull();
    expect(quotaAge(RESET, NOW)).toEqual({ kind: "now" });
    expect(quotaAge(instant("2026-11-15T11:59:30Z"), NOW)).toEqual({
      kind: "now",
    });
    expect(quotaAge(instant("2026-11-15T11:59:00Z"), NOW)).toEqual({
      kind: "ago",
      unit: "minutes",
      count: 1,
    });
    expect(quotaAge(instant("2026-11-15T09:59:00Z"), NOW)).toEqual({
      kind: "ago",
      unit: "hours",
      count: 2,
    });
    expect(quotaAge(instant("2026-11-12T09:59:00Z"), NOW)).toEqual({
      kind: "ago",
      unit: "days",
      count: 3,
    });
  });
});
