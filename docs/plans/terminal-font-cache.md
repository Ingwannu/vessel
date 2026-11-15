# Terminal painted-row and quiet-font cache delta

[Decision Log]
- Purpose: cursor phase changes retain painted rows, while quiet font-family
  changes invalidate them even when size/cell metrics are identical.
- Existing asset: the sole kit terminal grid and resolved code FontStack.
- Alternatives: rebuild all rows for blink, or key only by scalar metrics.
  The former repeats painting work; the latter keeps an obsolete font face.
- Choice: Rc row storage with a typed cache key carrying generation, appearance,
  cell geometry, viewport and the complete resolved family/fallback stack.
- Why: one renderer and one font stack identity govern measurement and drawing.
- Impact: blink changes only the cursor frame; output/appearance/viewport or
  code-stack changes replace rows. No new parser, setting store or renderer.

## Prerequisites and portable adoption

This commit is the portable terminal-owned delta. Its parent is a preparation
foundation, not an additional producer to adopt: the consumer already supplies
fonts::active(Typeface::Code) -> FontStack with Clone/PartialEq/Eq and the code
font face used by both measurement and draw. Keep that consumer-owned API.
The delta changes only existing terminal cache/frame files and its two tests;
it does not replace fonts.rs, font choices, settings or the font stack owner.

## Verification and limits

The source counterpart f0ea5489 executed strict all-target App Clippy0 and
row_reuse_tests2/0. Pre-fix quiet-font testcase executed0/1; its expectation
is retained. Both cursor row reuse and a held/quiet same-metrics code-stack
change are tested. These five code blobs and the remaining native source are
compared byte-for-byte with that exact checked counterpart after preparation.
The portable commit itself has no new build/runtime result. Consumer union
checks remain required; no whole package/F5 screen, physical font metrics,
visual capture or measured frame-time/CPU improvement is claimed here.
