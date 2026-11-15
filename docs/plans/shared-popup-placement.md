# Shared measured popup placement

[Decision Log]
- Purpose: menus and arbitrary-body popovers need to flip against a captured
  whole trigger, rather than against its bottom point.
- Existing assets: GPUI Anchored already owns a native child layout and
  point/corner snap. It cannot recover the trigger's top edge from a captured
  bottom point. W19's current popover likewise has point/snap placement only.
- Alternatives: a custom canvas, second popup renderer, delayed measurement
  state or area-specific geometry would split paint and input or duplicate
  policy. No external package or custom physics is necessary.
- Choice: popup::placement has one pure flip/clamp decision and one small
  Element that delegates layout/prepaint/paint to exactly one native child.
  Prepaint reads the real child size and current window viewport, computes
  one translation and prepaints the child there in the same cycle.
- Why: the child keeps its native paint and hit bounds together; no extra
  frame, focus/state cache or second renderer is introduced.
- Tradeoffs: this additive helper implements Start/End and Top/Bottom only.
  It does not implement center, horizontal flyouts, action/focus/lease policy,
  clipping, animation or presentation. Existing point-anchor paths remain.

Reference: Paseo 0.15.4 source1c00b365c9e0b74c1580eba33d905d693125ae21,
packages/app/src/components/ui/anchor.ts:24,35-128; the menu's full viewport
coordinates and options are menu/menu-overlay.tsx:164-177. Flip tests compare
raw available space against actual content size; they deliberately exclude
offset and edge. Clamp is max(low,min(high,raw)), so oversized content uses the
low edge without an invalid inverted clamp. G1 diff-pane.tsx:281-352 needs
whole-trigger Start260; W19 consumes the same primitive after review.

API: Alignment::{Start,End}, Side::{Top,Bottom},
Options{alignment,side,offset:Pixels,edge:Pixels};
compute(trigger Bounds,content Size,viewport Bounds,options) -> Placed{origin,side};
measured(trigger,options,one IntoElement child) -> Measured.
Callers pass logical offset4/edge8. The native wrapper uses the current GPUI
content viewport with origin0; the pure decision supports shifted viewports.
These are native layout measurements, not external unchecked wire data.

Tests freeze literal flip/alignment/exact-fit/equal-space/oversized/fractional
and nonzero-viewport outputs. The actual GPUI test uses two real child heights,
both alignments and an inset parent, reads actual prepaint bounds, clicks the
translated rectangle, and refuses old/unflipped positions. These tests are
installed, not executed yet. No menu/popover consumer is changed by this
producer, and no focused-field F5/OS/consumer capture pass is claimed.

This isolated source is based on announced e1b73c9e and starts only after the
previous F4 strict033 terminated. The unrelated glyph registry producer has
its own immutable check; all subsequent Rust execution remains serialized on
the build VM. V1 source+narrow and real input test results are owed.
