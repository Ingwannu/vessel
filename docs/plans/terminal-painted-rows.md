# Cursor-only frames reuse prepared and painted terminal rows

[Decision Log]
- Purpose: changing the cursor phase must reuse the static rows' preparation
  and paint callbacks, while output/appearance/geometry still invalidate rows.
- Existing assets: one kit terminal grid renderer, immutable shared row data,
  and the kit cached_view wrapper over GPUI's own retained prepaint/paint ranges.
  Reuse those; do not add another renderer, image snapshot or scene manager.
- Measurement: sharing row data alone preserved pixels but did not reduce the
  measured root-render-through-paint median: quiet3.259 to3.417ms, flood3.320
  to3.479ms under the same release profile/font/software-renderer fixture.
  These samples demonstrate no speedup. The existing canvas still prepares
  and paints every glyph on every cursor update.
- Alternatives: manually cache shaped lines (still paints all glyphs), stop
  cursor blinking, or copy scene internals. The toolkit already retains a
  cached child view's prepaint/paint ranges and handles window bounds/masks.
- Choice: one kit-owned retained row view plus the same renderer's cursor
  overlay. The aggregate view must be uncached: a dirty cached parent forces
  child refresh in this GPUI version. Notify only the row entity when its row
  identity, cell metrics, font size, background or full typed font stack changes.
- Why: GPUI's existing cache invalidation also checks bounds/content mask/text
  style. Holding the immutable row Rc prevents in-place mutation or pointer
  reuse from defeating row identity.
- Effects: cursor phase only repaints its overlay; output/theme/font/resize
  still draw correct rows. No application types or node operations in the kit,
  no area-local shared control copy and no unbounded cache.

Reference: Paseo runtime uses a blinking bar by default. This optimization does
not change its terminal behavior. Pinned GPUI view.rs prepaint_view/paint_view
reuses cached ranges when the child entity is unchanged; its refreshing flag
explains why the aggregate must not itself be a dirty cached boundary.

The branch deliberately holds the approved performance pair's source foundation
constant so another application-base change cannot masquerade as an optimization.
Final adoption is a portable owned delta on the announced app line with consumer
checks. First write a GPUI regression observing the real prepare/paint callbacks
across cursor-only updates, then implement retention, check and measure a new
release build. No improvement or new retained-paint proof is claimed yet.
