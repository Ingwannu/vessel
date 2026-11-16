import { describe, expect, it } from "vitest";

import { QuotaInstant, quotaClock, type QuotaClock } from "./clock";
import { quotaDurationText } from "./duration";

function instant(iso: string): QuotaInstant {
  const value = QuotaInstant.fromIso(iso);
  if (value === null) throw new Error("Invalid test timestamp");
  return value;
}

function reset(iso: string): Extract<QuotaClock, { kind: "reset" }> {
  const result = quotaClock(
    { reset: instant(iso), runOut: null, shortfall: null, basis: "current" },
    instant("2026-11-15T12:00:00Z"),
  );
  if (result.kind !== "reset")
    throw new Error("Expected a future reset fixture");
  return result;
}

describe("quota duration in the viewer's language", () => {
  it("formats the already-floored hours and minutes in English and Korean", () => {
    const clock = reset("2026-11-15T14:10:59Z");
    expect(quotaDurationText(clock.parts, "en-US")).toBe("2h 10m");
    expect(quotaDurationText(clock.parts, "ko-KR")).toBe("2시간 10분");
  });

  it("retains the two largest units without rounding up a remaining day", () => {
    const clock = reset("2026-11-18T14:00:00Z");
    expect(quotaDurationText(clock.parts, "en-US")).toBe("3d 2h");
    expect(quotaDurationText(clock.parts, "ko-KR")).toBe("3일 2시간");
  });

  it("does not invent seconds or turn the final partial minute into an elapsed reset", () => {
    const clock = reset("2026-11-15T12:00:59Z");
    expect(quotaDurationText(clock.parts, "en-US")).toBe("0m");
    expect(quotaDurationText(clock.parts, "ko-KR")).toBe("0분");
  });

  it("keeps the reference's omitted zero minor unit", () => {
    const clock = reset("2026-11-15T13:00:00Z");
    expect(quotaDurationText(clock.parts, "en-US")).toBe("1h");
    expect(quotaDurationText(clock.parts, "ko-KR")).toBe("1시간");
  });
});
