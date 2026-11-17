import { describe, expect, it } from "vitest";

import { QuotaInstant, quotaClock, type QuotaClock } from "./clock";
import { quotaPercentText, quotaResetClock } from "./format";
import { QuotaPercent } from "./values";

function instant(iso: string): QuotaInstant {
  const value = QuotaInstant.fromIso(iso);
  if (value === null) throw new Error("Invalid fixture timestamp");
  return value;
}

const NOW = instant("2026-11-15T12:00:00Z");
const DEVICE_CLOCK = new Intl.DateTimeFormat("en-US", {
  hour: "numeric",
  minute: "2-digit",
  hourCycle: "h23",
  timeZone: "America/Los_Angeles",
});

function reset(iso: string): QuotaClock {
  return quotaClock(
    { reset: instant(iso), runOut: null, shortfall: null, basis: "current" },
    NOW,
  );
}

function percent(value: number): QuotaPercent {
  const parsed = QuotaPercent.from(value);
  if (parsed === null) throw new Error("Invalid fixture percentage");
  return parsed;
}

describe("localized quota values and reset clock hints", () => {
  it("formats reported zero, rounded percent and overflow after the one clamp", () => {
    expect(quotaPercentText(percent(0), "en-US")).toBe("0%");
    expect(quotaPercentText(percent(12.6), "en-US")).toBe("13%");
    expect(quotaPercentText(percent(125), "ko-KR")).toBe("100%");
    expect(quotaPercentText(percent(50), "tr-TR")).toBe("%50");
  });

  it("uses the device's clock within a day, rather than the language's default cycle", () => {
    expect(
      quotaResetClock(reset("2026-11-15T22:10:00Z"), "ko-KR", DEVICE_CLOCK),
    ).toEqual({ kind: "time", time: "14:10" });
  });

  it("adds a localized weekday at exactly one day using that same time zone", () => {
    const clock = reset("2026-11-16T12:00:00Z");
    expect(quotaResetClock(clock, "en-US", DEVICE_CLOCK)).toEqual({
      kind: "weekday",
      day: "Mon",
      time: "04:00",
    });
    expect(quotaResetClock(clock, "ko-KR", DEVICE_CLOCK)).toEqual({
      kind: "weekday",
      day: "월",
      time: "04:00",
    });
  });

  it("adds the local date at a week or later, including the previous UTC date", () => {
    const clock = reset("2026-11-22T12:00:00Z");
    const honoluluClock = new Intl.DateTimeFormat("en-US", {
      hour: "numeric",
      minute: "2-digit",
      hourCycle: "h23",
      timeZone: "Pacific/Honolulu",
    });
    expect(quotaResetClock(clock, "en-US", honoluluClock)).toEqual({
      kind: "date",
      day: "Nov 22",
      time: "02:00",
    });
    const next = reset("2026-11-23T01:00:00Z");
    expect(quotaResetClock(next, "ko-KR", honoluluClock)).toEqual({
      kind: "date",
      day: "11월 22일",
      time: "15:00",
    });
  });

  it("never borrows the reset's exact clock for a projected run-out", () => {
    const risk = quotaClock(
      {
        reset: instant("2026-11-16T12:00:00Z"),
        runOut: instant("2026-11-15T14:00:00Z"),
        shortfall: percent(5),
        basis: "current",
      },
      NOW,
    );
    expect(risk.kind).toBe("risk");
    expect(quotaResetClock(risk, "en-US", DEVICE_CLOCK)).toBeNull();
  });

  it("shows no exact clock for missing, resetting or retained passed observations", () => {
    const passed = quotaClock(
      { reset: NOW, runOut: null, shortfall: null, basis: "retained" },
      NOW,
    );
    for (const clock of [
      { kind: "unreported" } as const,
      reset("2026-11-15T12:00:00Z"),
      passed,
    ]) {
      expect(quotaResetClock(clock, "en-US", DEVICE_CLOCK)).toBeNull();
    }
  });
});
