// Layout can queue a scroll before ResizeObserver is delivered. That
// event is not evidence that a reader left the end; a move upward in
// unchanged content is. Keep this decision independent of DOM timing.

/** Geometry read at one layout/scroll boundary. */
export interface EndView {
  top: number;
  height: number;
  width: number;
  content: number;
}

/** How close to the end counts as reading it, in pixels. */
export const NEAR_END = 48;

/** A layout needs correction, the reader is near the end, or reading up. */
export function followAfterScroll({
  before,
  after,
  following,
}: {
  before: EndView;
  after: EndView;
  following: boolean;
}): "end" | "near" | "away" {
  const near = after.content - after.top - after.height < NEAR_END;
  const layout =
    before.height !== after.height ||
    before.width !== after.width ||
    before.content !== after.content;
  // A move outside the near-end band still wins over simultaneous
  // layout. A browser clamp inside that band retains end intent.
  if (following && layout && (after.top >= before.top || near)) return "end";
  return near ? "near" : "away";
}
