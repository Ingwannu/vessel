import { describe, expect, it } from "vitest";

import type { CheckRun, CheckState } from "../../../wire/proto";
import {
  checksSummary,
  circumference,
  duration,
  segments,
  summaryParts,
  timing,
} from "./checks";

function run(name: string, state: CheckState): CheckRun {
  return { name, state };
}

const START = Date.parse("2026-11-19T10:00:00Z");
const at = (seconds: number): string =>
  new Date(START + seconds * 1000).toISOString();

describe("the checks section (checks.rs)", () => {
  it("groups in Paseo's order and keeps each group's own order", () => {
    const runs = [
      run("lint", "passed"),
      run("docs", "skipped"),
      run("build", "failed"),
      run("test", "pending"),
      run("deploy", "cancelled"),
      run("e2e", "passed"),
      run("new", "unknown"),
    ];
    const read = checksSummary(runs);
    expect(read.outcome, "a failure outranks a run").toBe("failure");
    expect(read.total, "skipped ones count in the total").toBe(7);
    expect(summaryParts(read)).toEqual([
      { presentation: "failure", count: 1 },
      { presentation: "pending", count: 2 },
      { presentation: "success", count: 2 },
      { presentation: "ignored", count: 2 },
    ]);
    expect(read.groups[2]?.checks.map((check) => check.name)).toEqual([
      "lint",
      "e2e",
    ]);
  });

  it("makes the outcome the worst thing happening", () => {
    expect(checksSummary([]).outcome).toBe("none");
    const running = [run("a", "passed"), run("b", "pending")];
    expect(checksSummary(running).outcome).toBe("pending");
    const done = [run("a", "passed"), run("b", "skipped")];
    expect(checksSummary(done).outcome).toBe("success");
    expect(
      checksSummary([run("a", "skipped")]).outcome,
      "all skipped reads as passed, as Paseo's",
    ).toBe("success");
  });

  it("draws the ring as one segment per part, with gaps", () => {
    const whole = circumference();
    expect(whole).toBeCloseTo(2 * Math.PI * 8.5, 4);
    expect(segments([], 0)).toEqual([]);
    const one = segments([{ presentation: "success", count: 3 }], 3);
    expect(one).toHaveLength(1);
    expect(one[0]?.length, "one part fills the ring").toBeCloseTo(whole, 4);
    const two = segments(
      [
        { presentation: "failure", count: 1 },
        { presentation: "success", count: 3 },
      ],
      4,
    );
    const quarter = whole / 4;
    expect(two[0]?.start).toBeCloseTo(0, 4);
    expect(two[0]?.length).toBeCloseTo(quarter - 2, 4);
    expect(two[1]?.start, "starts after the whole share").toBeCloseTo(
      quarter,
      4,
    );
    expect(two[1]?.length).toBeCloseTo(3 * quarter - 2, 4);
    const tiny = segments(
      [
        { presentation: "failure", count: 1 },
        { presentation: "success", count: 99 },
      ],
      100,
    );
    expect(tiny[0]?.length, "never shorter than the minimum").toBeCloseTo(
      1.5,
      4,
    );
  });

  it("times a run from its start to its end, or to now", () => {
    const now = START + 150_000;
    expect(timing(run("build", "passed"), now), "no start, no timing").toBe(
      null,
    );
    const done: CheckRun = {
      ...run("build", "passed"),
      started_at: at(0),
      completed_at: at(90),
    };
    expect(timing(done, now)).toEqual({ running: false, seconds: 90 });
    const running: CheckRun = { ...run("test", "pending"), started_at: at(0) };
    expect(timing(running, now)).toEqual({ running: true, seconds: 150 });
    const backwards: CheckRun = { ...done, completed_at: at(-1) };
    expect(timing(backwards, now), "an end before its start").toBe(null);
  });

  it("truncates toward zero as chrono does, and refuses a broken time", () => {
    const near: CheckRun = {
      ...run("build", "passed"),
      started_at: "2026-11-19T10:00:00.500Z",
      completed_at: "2026-11-19T10:00:00Z",
    };
    const read = timing(near, START);
    expect(read, "half a second early is still 0 s").toEqual({
      running: false,
      seconds: 0,
    });
    expect(Object.is(read?.seconds, -0)).toBe(false);
    const broken: CheckRun = { ...run("build", "passed"), started_at: "soon" };
    expect(timing(broken, START)).toBe(null);
    const brokenEnd: CheckRun = {
      ...run("build", "passed"),
      started_at: at(0),
      completed_at: "later",
    };
    expect(timing(brokenEnd, START), "not read as still running").toBe(null);
    const nulls: CheckRun = {
      ...run("test", "unknown"),
      started_at: at(0),
      completed_at: null,
    };
    expect(timing(nulls, START + 5_000)).toEqual({
      running: true,
      seconds: 5,
    });
  });

  it("writes a duration as Paseo's server does", () => {
    expect(duration(0)).toBe("0s");
    expect(duration(45)).toBe("45s");
    expect(duration(90)).toBe("1m 30s");
    expect(duration(120)).toBe("2m");
    expect(duration(3900)).toBe("1h 5m");
    expect(duration(3601)).toBe("1h 1s");
    expect(duration(90_000), "hours do not roll into days").toBe("25h");
  });
});
