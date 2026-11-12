// Which items of a long list to draw: those within a margin of the
// viewport. The others keep their space as two spacers, from the heights
// measured so far and a guess for the rest
// (features/session/use-window.ts draws the timeline this way).
//
// Pure arithmetic, tested on its own; nothing here touches the DOM.

/** The items to draw, `start` to `end` (exclusive), and the space the
 * items before and after them take, in pixels. */
export interface Span {
  start: number;
  end: number;
  before: number;
  after: number;
}

/** The mean of the heights measured, or `fallback` before any. */
export function guess(
  heights: ReadonlyMap<string, number>,
  fallback: number,
): number {
  if (heights.size === 0) {
    return fallback;
  }
  let sum = 0;
  for (const height of heights.values()) {
    sum += height;
  }
  return sum / heights.size;
}

/** Where each item starts, and after the last, where the list ends:
 * `keys.length + 1` offsets. */
export function offsets(
  keys: readonly string[],
  heights: ReadonlyMap<string, number>,
  fallback: number,
): number[] {
  const estimate = guess(heights, fallback);
  const result = new Array<number>(keys.length + 1);
  let at = 0;
  result[0] = 0;
  keys.forEach((key, index) => {
    at += heights.get(key) ?? estimate;
    result[index + 1] = at;
  });
  return result;
}

/** The first index whose item ends below `y` (binary search over
 * `starts`, which `offsets` gives in increasing order). */
function firstEndingBelow(starts: readonly number[], y: number): number {
  let low = 0;
  let high = starts.length - 1;
  while (low < high) {
    const middle = Math.floor((low + high) / 2);
    if ((starts[middle + 1] ?? Infinity) > y) {
      high = middle;
    } else {
      low = middle + 1;
    }
  }
  return low;
}

/** The items overlapping `top` to `top + height`, widened by `margin`
 * each way. */
export function span(
  starts: readonly number[],
  top: number,
  height: number,
  margin: number,
): Span {
  const count = starts.length - 1;
  const total = starts[count] ?? 0;
  if (count <= 0) {
    return { start: 0, end: 0, before: 0, after: 0 };
  }
  const start = Math.min(firstEndingBelow(starts, top - margin), count - 1);
  let end = start;
  const bottom = top + height + margin;
  while (end < count && (starts[end] ?? total) < bottom) {
    end += 1;
  }
  end = Math.max(end, start + 1);
  const before = starts[start] ?? 0;
  const after = total - (starts[end] ?? total);
  return { start, end, before, after };
}
