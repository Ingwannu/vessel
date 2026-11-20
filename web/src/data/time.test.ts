import { describe, expect, test } from "vitest";

import { compactTimeAgo } from "./time";

const MINUTE = 60_000;
const HOUR = 60 * MINUTE;
const DAY = 24 * HOUR;

describe("Paseo's compact age", () => {
  const at = new Date(2026, 0, 15, 12, 0, 0);

  test("under a minute is now, then minutes, hours and days", () => {
    expect(compactTimeAgo(at, new Date(at.getTime() + 59_000)).label).toBe(
      "now",
    );
    expect(compactTimeAgo(at, new Date(at.getTime() + MINUTE)).label).toBe(
      "1m",
    );
    expect(compactTimeAgo(at, new Date(at.getTime() + 5 * MINUTE)).label).toBe(
      "5m",
    );
    expect(compactTimeAgo(at, new Date(at.getTime() + 2 * HOUR)).label).toBe(
      "2h",
    );
    expect(compactTimeAgo(at, new Date(at.getTime() + 3 * DAY)).label).toBe(
      "3d",
    );
    expect(compactTimeAgo(at, new Date(at.getTime() + 6 * DAY)).label).toBe(
      "6d",
    );
  });

  test("a week or more is the month and day", () => {
    expect(compactTimeAgo(at, new Date(at.getTime() + 7 * DAY)).label).toBe(
      "Jan 15",
    );
  });
});
