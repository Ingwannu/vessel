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

/** Where a row is put when the window goes to it: its top just under the
 * view's top (the outline's jump to a prompt), or the thing found in the
 * view's middle (find's match). */
export type Align = "start" | "center";

/** The gap left above a row put at the view's top, in pixels. */
export const INSET = 16;

/** The `scrollTop` that puts item `index` at `align` in a view `view`
 * pixels high, from the offsets as `offsets` estimated them. The drawn
 * row then says how far off the estimate was (`offBy`). */
export function scrollFor(
  starts: readonly number[],
  index: number,
  view: number,
  align: Align,
): number {
  const top = starts[index] ?? 0;
  const bottom = starts[index + 1] ?? top;
  switch (align) {
    case "start":
      return Math.max(0, top - INSET);
    case "center":
      return Math.max(0, (top + bottom) / 2 - view / 2);
  }
}

/** How far the content must scroll (positive: down) for a box drawn `top`
 * pixels below the view's top edge, `height` high, to sit at `align` in a
 * view `view` pixels high. A box taller than the view is put at its top,
 * as `center` would hide its start. */
export function offBy(
  top: number,
  height: number,
  view: number,
  align: Align,
): number {
  if (align === "start" || height > view) {
    return top - INSET;
  }
  return top + height / 2 - view / 2;
}

/** Whether the drawn items (`span`, over `starts`) cover a view `height`
 * pixels high scrolled to `top`: when they do not, part of the view shows
 * the spacers' empty space until the window is worked out again. */
export function covers(
  starts: readonly number[],
  drawn: Span,
  top: number,
  height: number,
): boolean {
  const from = starts[drawn.start] ?? 0;
  const to = starts[drawn.end] ?? from;
  return drawn.start === drawn.end || (from <= top && to >= top + height);
}
