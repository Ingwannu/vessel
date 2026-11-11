import { describe, expect, it } from "vitest";

import { guess, offsets, span } from "./window";

const keys = Array.from({ length: 1000 }, (_unused, n) => `k${String(n)}`);

describe("a list's window", () => {
  it("guesses unmeasured heights from the measured ones", () => {
    expect(guess(new Map(), 40)).toBe(40);
    expect(
      guess(
        new Map([
          ["a", 10],
          ["b", 30],
        ]),
        40,
      ),
    ).toBe(20);
    const starts = offsets(["a", "b", "c"], new Map([["b", 50]]), 40);
    // a and c are guessed at b's 50.
    expect(starts).toEqual([0, 50, 100, 150]);
  });

  it("draws the items in view and a margin, and keeps the rest's space", () => {
    const starts = offsets(keys, new Map(), 20);
    expect(starts.at(-1)).toBe(20_000);
    // Rows 100 to 129 are in a 600 px view at 2,000 px; 200 px of margin
    // adds 10 rows each way.
    const window = span(starts, 2000, 600, 200);
    expect(window).toEqual({ start: 90, end: 140, before: 1800, after: 17200 });
    expect(
      window.before + (window.end - window.start) * 20 + window.after,
    ).toBe(20_000);
  });

  it("stays inside the list at its ends", () => {
    const starts = offsets(keys, new Map(), 20);
    expect(span(starts, 0, 600, 200)).toEqual({
      start: 0,
      end: 40,
      before: 0,
      after: 19_200,
    });
    const end = span(starts, 19_400, 600, 200);
    expect(end.end).toBe(1000);
    expect(end.after).toBe(0);
    // Scrolled past the end (the list just shrank): the last item.
    expect(span(starts, 50_000, 600, 0)).toMatchObject({
      start: 999,
      end: 1000,
    });
    expect(span([0], 0, 600, 200)).toEqual({
      start: 0,
      end: 0,
      before: 0,
      after: 0,
    });
  });

  it("follows measured heights", () => {
    const heights = new Map(keys.slice(0, 10).map((key) => [key, 100]));
    const starts = offsets(keys, heights, 20);
    // Ten rows of 100, the rest guessed at their mean, 100.
    expect(starts[10]).toBe(1000);
    expect(span(starts, 950, 100, 0)).toMatchObject({ start: 9, end: 11 });
  });
});
