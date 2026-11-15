# ADR 0059: The mark's multiline drawing on screen, derived, not scaled

Status: accepted for implementation, 2026-11-14, on the owner's request
("Multiline Lettering, Multiline Logo"); the owner chooses among the
study's variants, and the default below is the study's recommendation.

## Context

The owner asked for the brand to be on the screen and not only in the
title bar: the composer of a new session inside the mark's V drawn large,
the star above it; the same V framing an open session; the star going to
the agent's reply on the first message and becoming its avatar, a circle
holding the agent's own mark; the multiline character stronger, read as
circuit traces reduced to parallel lines.

The mark (`docs/design/brand/`) is a filled outline: a V of nested loops
(four per band, eight across an arm) with a gap of 1.33 lines, every turn
concentric, a small ring where a line ends, and a four-pointed star. Its
measured ratios are in `docs/design/brand/README.md`, "Multiline rules".

Constraints: the text over the drawing must stay as readable as it is
today; the app is still by default (no cost when nothing moves); a mark
must be sharp at scale 1, 1.25, 1.5 and 2; every screen works from 360
points wide; GPUI stays out of `view`, and the kit knows no brand.

## Options

1. **The owner's mark scaled up, untouched.** At a 1280-point window the
   V is 930 points wide: its lines become 12-point bands with 16-point
   gaps, heavy for a background, and its arms pass under the heading and
   the pickers. Measured over the real screens, the pickers' text fell
   from 4.58:1 to 3.07:1 at the stronger ink (below 4.5:1).
2. **A derived drawing in the mark's construction** (nested loops on one
   pitch, turns at (k + ½) pitches, 60° arms, the owner's star at a third
   of the V's width), its pitch chosen for the screen, its arms placed to
   clear every text box.
3. **The same lines as circuit traces** (45° turns, ends fanned out to
   twice the pitch into the mark's ring pads). Reads as a circuit; at a
   size where it frames a composer it reads less as the mark's V.

Painting, for whichever drawing:

- GPUI paths (`PathBuilder`, lyon): tessellated on every paint, edges
  from at most 4× multisampling (five coverage levels; steps visible on
  60° lines in an 800 % crop).
- The agent marks' method (ADR 0048): the vector rasterised by GPUI's
  own renderer at exactly the device pixels it covers, painted 1:1, the
  raster cached by content.

## Decision

Option 2 for the screens, at a quiet ink (dark: the foreground at 7 %;
light: the brand colour at 10 %). Option 3's language goes to the window
frame and the window controls, where traces and pads are at home.

- `view::brand` owns the geometry and the motion, with tests against the
  real screens' measured boxes: `ratios` (the mark's numbers and the
  pitch for a width), `geometry` (offset lines with concentric turns,
  loops, the star and its relaxing into a circle), `layout` (where each
  screen's drawing goes and how it degrades), `motion` (the first
  message's journey), `shapes` (what the kit paints).
- `kit::vector` paints shapes it is given (points, a stroke or a fill, a
  share of one ink) by the agent marks' method: no resampling between the
  vector and the screen; at rest one cached texture; a raster pushed out
  of the small cache is dropped from GPUI's atlas.
- `app::brand` is the shell's: the screens report their boxes while they
  are laid out, the backdrop draws in the content panel behind whichever
  screen is shown, in the same frame. The new-session screen gives its
  column one listener; the session gives its body one; the avatar is one
  call at the head of an agent message.
- Degrading: a new session wide enough draws the whole V; narrower (the
  arms would cross the heading or the pickers), the star above the
  heading and the V's bottom under the pickers; an open session draws as
  many loops a side as its margin holds after the air beside the column
  (four, three, two), none below two, its arms leaning in as far as the
  margin allows and at most 20° (the owner's sketch of the open session).
- Motion: one bounded transition on the first message (the move 300 ms,
  star to circle 150 ms, the agent's mark 150 ms, the Codex app's curve,
  all read from one place); one step per presented frame at the display's
  own rate while it runs.
  Reduced motion, a software renderer or animations turned off show the
  end state at once. Nothing moves otherwise.

## Why

The drawing reads as the mark at any size because it keeps the mark's
ratios, not its pixels; every line stays separate at scale 1 (a 7-point
pitch at 1280: lines 3, gaps 4); no line passes under text, so contrast
is today's by construction rather than by measurement each time. The
raster method is the one the agents' marks already use, so the app has
one way to draw a vector sharply.

## Consequences

- The 40-point mark above the new session's heading is removed: the
  drawing is the mark.
- The drawing depends on boxes the screens report; a screen that reports
  nothing shows no drawing (the board, settings), which is intended.
- A layout change rasterises the drawing once, only the area it fills; a
  journey rasterises about eighteen frames, of which the cache keeps two.
  Measured against a build without the drawing (the lead's method:
  release build, software renderer, `top` over five seconds): no CPU at
  rest beyond it, about 5 MB more memory on the new session and 15 MB on
  the open session (`docs/plans/backend-m12-brand.md`).
- The owner may still choose option 1 or 3; both are drawn in the study
  and option 3's geometry is the same module's to add.
