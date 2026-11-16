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

Pre-compile source correction: translate from the wrapper's native origin,
matching GPUI Anchored, rather than from the child's relative origin. Otherwise
the placement helper would cancel the existing popup::appear local rise and
accidentally own animation. The contract is a margin-free child, as with GPUI's
point anchor; caller-local relative motion stays inside the positioned wrapper.
An additional actual GPUI case expects the caller's literal4 local offset to
survive in both painted/prepaint and hit bounds. No animation engine or timer
is added; this test is still unexecuted.

Independent reference oracle: Node24.22.1 directly imported the unchanged
Paseo anchor.ts (type-only React Native import erased; no app/runtime boot).
Eleven literal cases passed against the original computePosition, including
both alignments/sides, equal space, exact raw fit despite offset, oversized
content, fractional points, shifted viewport and the second real-child height.
The loaded file bytes independently equal the pinned1c00 Git blob.
Evidence: placement-source-oracle.mjs and its retained source-oracle JSON.
This validates the expected numeric cases against the actual reference; it is
NOT execution of the Rust helper, actual GPUI input, OS motion or consumer UI.

Preflight keeps the nine literal policy cases in two named fixture groups,
separate from the one compute/assertion runner, to stay below strict function
length without dropping numeric cases. Terminate the measurement callback's
unit assignment explicitly. These are pre-compile test-source corrections;
no Rust pass is inferred from formatting or the independent TS oracle.


Resumed source is merged onto announced package base13 before its first Rust
check. The previous F4 strict has terminated; a later Main strict is now green,
with Main runtime tests still queued separately. This geometry producer changes
no menu/popover consumer or existing focused root. Native source/runtime checks
and V1 adoption remain owed; no earlier source-oracle result is a Rust pass.

The first exact058 priority strict reached Kit and returned one unused-results
diagnostic: child prepaint reports an Option<FocusHandle>. GPUI AnyElement
already records that focus in Window; bind its report without focusing/blurring
or adding a focus owner. The geometry and all literal tests remain unchanged.
This failed run is retained; subsequent strict/tests use ordinary lanes under
the coordinator's failure rule. No compiler or GPUI runtime pass is inferred.

The corrected exact1b2552e6 Kit all-target strict completed in
shell2-narrow-20261116T013211Z-1b2552e6: fonts0/cargo0, with actual Kit Checking.
The queue wait is not compilation time; native-cargo itself took10 seconds.
The next natural boundary merges announced base14.1 b54abbdc before runtime.
All three placement source/test files remain byte-identical to1b; the popup
module registration is retained alongside the receiving base's exports.
This prior-head strict is not relabelled as exact receiving-union strict or
runtime proof. Actual placement GPUI tests on that receiving source are next;
V1/consumer adoption, consumer images and full F4 remain separate and owed.

## Exact runtime result and quota-stop handoff

Receiving source3958ad382480d75657c7527e26d234b5590fd73f completed ordinary
shell2-narrow-20261116T025223Z-3958ad38: fonts0/cargo0, actual fresh GPUI and
Kit Compiling, three passed/zero failed/zero ignored (271 filtered out).
The policy tests execute nine literal cases plus the shifted viewport; the
actual GPUI case executes both alignments, heights80/120, parent inset20 and
caller-local offset4, checks native prepaint bounds, hits the translated child
and rejects the old origin/unflipped edge. This is not a full Kit lib run.

Prior exact1b strict remains prior-head evidence; all three placement source
and test blobs are identical in395, while the announced receiving base changed.
Do not label it exact395 Clippy. No consumer menu/popover was migrated, and
this module owns no focus, lease, action or new surface. V1 bounded shared
review and lead receiving gate remain separate; actual consumer images are
still owed by G1/W19 after integration. No Linux F5/OS/full F4 claim follows.

Quota stop: no new Cargo/merge/control is started. This result document changes
no native source; source395 is the actual runtime snapshot. The final shared
packet includes the qualified strict/result logs and source identity. Resume
from Main docs/plans/wp-f4-sidebar-resume.md on backend/m12-wp-sidebar and
inspect completed025223 before submitting anything; do not replay it. Main
changed receiving strict/runtime and all other WIP controls remain deferred.
