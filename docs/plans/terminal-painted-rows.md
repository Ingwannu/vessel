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
- Choice: one active kit-owned retained row view plus the same renderer's cursor
  overlay. The aggregate view must be uncached: a dirty cached parent forces
  child refresh in this GPUI version. Replace the immutable row child only when
  row identity, cell metrics, font size, background or the full typed font stack
  changes. GPUI then paints the new child in that same frame; the previous child
  is released through the toolkit's existing entity/scene lifetime.
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

Resume: the kit control now owns one cached static-row entity and an uncached
cursor overlay, both using the original drawing helpers. The app's aggregate
grid is deliberately uncached. Tests observe actual preparation/paint callbacks
and cover cursor reuse, output/background/font-size/metrics invalidation, and
full typed code-stack replacement with fixed metrics. The first baseline probe
had an incorrect test-only Strike variant; the corrected baseline is retained
as66bd6963. Its original queued compile remains untouched. No semantic failing
baseline or passing corrected runtime is claimed until the remote logs exist.

The initial base13 checks executed actual terminal App regressions18/0/1 ignored,
but the new kit fixtures failed two assertions. Updating a child with notify
inside Render only invalidates its cache on the next draw, leaving one stale
frame. Immutable child replacement fixes that boundary without another cache
or renderer. The font key snapshot is now also the font used for shaping,
instead of rereading a mutable global after choosing its key. Fixtures pass a
typed font snapshot through that same private path without changing process-wide
fonts concurrently. The App's existing published-font-API regression remains.
Cursor tests additionally require actual overlay paint progress on both blink
directions; equal static-row counters alone could conceal an absent draw.
Strict-check style/Debug/borrow-only constructor issues are corrected without
waivers. Actual corrected checks and release measurements remain owed.

Corrected8f15853b passed selected App18/0/1 ignored and Kit5/0/0 (four new
retained-paint fixtures plus one existing diff-colour test). Strict all-target
Kit/App Clippy also passed. Test log prefixes0f32e6810e6bb31d/d0be867d3b65eb33
identify the actual executions, not the earlier failing suite. The corrected
baseline66bd executed0/1 with row preparation/paint increasing on cursor update,
so that is semantic red; the original Strike typo remains a separate compiler
failure. The two App font fixtures now serialize access to their shared global
font snapshot without changing assertions or deadlines.

The controlled measurement sourcef640 has identical terminal renderer/control,
App grid adapter and GPUI cached-view implementation bytes to the checked
base13 source; its own four control tests and fixed ordinary release export are
submitted separately. The previous before/after pair demonstrated no speedup.
One native whole on the current base13 consumer and new same-profile release
measurement/inspected captures remain owed. No D1 frame/input/IME or whole F5
settings approval follows from these renderer checks.


## Completed enclosing verification and release comparison

Exacte11e2872 passed the one ordinary native whole
appconv-paint-tests-20261116T011444Z-e11e2872: all six planned steps executed
with rc0, native tests1621/0/2. Every raw step hash was independently recomputed.
The two ignored tests are existing explicit display fixtures, not skipped
retained-paint regressions. Quality checks710 files/13 existing size exceptions,
four boundaries/seven owned externals/two name rules; token counts417/74/45,
none new. Root sources/manifests/locks are unchanged by this native slice.

The matching controlled release exportf6400cd5 passed using the same fixed
recipe/profile/features/toolchain and approved shared-root hashes as6b23a397.
Its renderer/control/App adapter and GPUI cached-view bytes match the checked
base13 implementation. Receiver verified full app hash, glibc2.39 requirement,
UID/GID1000 and mode0500. The common source foundation isolates this change
from unrelated app-base changes. No signing, feed, push or local Rust build.

Fresh focused sequential trials use one private fake TUI/display at1740x900,
dark/en/scale1, software Vulkan, sidebar closed and motion enabled. Baseline
idle30s:59 frames, median3384us/p954842us; corrected60 frames,
median2218us/p953505us. Flood20s:46 frames each, median3430→2249us,
p956279→6934us. Median decreases34.46%/34.43% in these samples; the flood
tail did not improve. Process CPU remains similar (83.16→82.23% idle,
95.70→96.64% flood of one core), so no general CPU reduction is claimed.
The measure is root-render-through-paint CPU elapsed, not hardware GPU time,
input latency or a universal benchmark. The earlier pair with no speedup and
an initial unfocused zero-idle-draw interval remain preserved and excluded.

All12 new captures were opened personally. Ready/idle/flood static terminal
ROI(8,114)-(1728,864) has zero changed pixels in each comparison. The cursor
below that ROI is covered by actual overlay progress tests, not this pixel
comparison. Both measured apps were stopped only after exact saved PID,
executable/start-tick/account verification; no live preview/real agent was used.
Detailed hashes/raw measurements are in the review packet. This final follow-up
changes documentation only. Native renderer review is owed; D1 transport,
selection/input/IME, physical OS input and whole F5 UI remain unqualified.
