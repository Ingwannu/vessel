# Content-fit sheet adapter

[Decision Log]
- Purpose: reference menu pages have different natural heights and fit their
  content up to80 percent of the viewport, with pan-down dismissal and a45
  percent backdrop. A fixed percentage or a second DialogState cannot express
  that while preserving the menu's existing focus and opening admission.
- Existing assets: sheet::snap owns the spring, sampled pointer velocity,
  over-drag resistance and content-scroll handoff. Its public authored
  SnapPoints requires two unequal percentages; dialog's private adapter owns
  separate focus and cannot be composed inside MenuState.
- Alternatives: copying dialog's frame, equal authored stops, another gesture
  controller, or area-specific sheet paint would duplicate policy or change
  existing defaults. No external package is required.
- Choice: extend the existing physics with an internal measured single-open
  configuration and add one focus-agnostic natural-content frame adapter.
  Preserve authored60/90 and every existing Dialog API/default/renderer.
- Why: one native child supplies actual layout height; the same physics and
  pointer/scroll adapter move it. The caller retains its own lease, focus,
  overlay ordering, action policy and natural header/row content.
- Impact: the adapter admits input through a caller-supplied predicate before
  mutation. It cannot grant an action, own a second focus or invent a second
  per-open token. Existing sheet listeners retain their legacy behavior.

Reference: Paseo components/ui/menu/menu-surface.tsx, whole, especially
content-sized dynamic sizing, maxDynamicContentSize windowHeight*.8,
backdropOpacity .45 and pan-down-to-close in MenuSheetSurface. Source
menu-sheet-layout adds safe-area bottom plus spacing1; native safe-area facts
remain caller-owned. Motion and direct manipulation use existing sheet::snap.

Coordinator assigned this isolated adapter after W19 confirmed no checked
public stateless frame. No Dialog paint is copied; if common paint must move,
it will be extracted unchanged in a separate move-only commit first. The
initial adapter is geometry/input only; Menu's existing surface renderer will
supply its frame and shared popup::sheet_scrim45 color.

Resume: docs skeleton only, no Rust source installed or check claimed. Work
on the announced6eb base and keep all compilation on the build machine,
serialized after the already-queued policy check. Source/narrow/native input
proof and V1/lead handoff remain owed.

Physics source checkpoint: keep public authored SnapPoints validation exactly
as before; internal Positions::ContentFit is a separate measured configuration.
Both compact/expanded semantic samples resolve to its one open position, so
nearest-stop, spring, sampled drag and content-scroll handoff use their existing
implementations. Existing default positions retain their exact literal branch.
Retargeting content cancels a pan and settles that same spring from its sampled
position; unchanged geometry leaves an active gesture untouched. Invalid layout
retains prior state. New tests drive these actual methods and preserve every
old authored-configuration test unchanged. No renderer/input adapter yet; all
new Rust is uncompiled and no checked producer is claimed.

Input source checkpoint: listener_when delegates to the same native gesture
listener and requires the caller's admission predicate before press, pan,
scroll/capture, release and its callback. Legacy listener delegates with no
new token or policy. A host must supply its existing opening admission and
invalidate it on replacement/actual close; the adapter creates no lease.
This source is uncompiled and native retained-input tests remain owed.

Frame source checkpoint: content_fit::State contains only the shared motion
and native ScrollHandle. Its transparent Fitted Element caps one real child,
measures it during prepaint and translates it with the same sampled spring;
child paint and input bounds move together. A host resets this geometry on
explicit reopen, and supplies the existing admission before measurement,
paint and every native gesture mutation. No focus, key handler, root menu path,
second opening token, surface paint or Dialog code is installed here.

Caller requirements: pass the validated80-percent source cap, give the panel a
bounded min-height-zero scroller tracking State::scroll, provide its natural
handle/header, and paint the backdrop using existing popup::sheet_scrim45.
State::input attaches only to handle/list regions, never editable fields.
A pan-down callback retains the same open control until the owner's actual
close ACK removes it, matching the existing Dialog refusal behavior. Native
safe-area/breathing-room content remains caller-owned. There is no copied
Dialog frame paint to extract in this geometry-only adapter.

Installed actual GPUI tests read fractional/capped native bounds and hit them,
retain render A while the host resets geometry/opening B before redraw, require
A to leave B motion/scroll/dismissal untouched, then require fresh B pan-down
and refused-close retention. Pure tests also drive the shared content scroll
handoff. All remain UNEXECUTED until the serialized remote narrow run; this
checkpoint is not an approved producer or full menu-sheet/UI claim.

A further actual GPUI fixture uses a1200-point body in the300-point fitted
panel, tracking the same native ScrollHandle: upward content input scrolls100,
a50-point downward pull leaves50 scrolled and the panel fixed, then only the
remaining pull after offset zero may request close. This complements the pure
engine cases; it too is installed, not executed. Child actions/focus must still
use the host's lease: the geometry adapter does not authorize arbitrary child
callbacks or claim complete menu keyboard accessibility.

Retained-frame fixture queues the new owner render explicitly via notify,
while asserting dispatch still uses A before that redraw. Only after A's
input is checked does the fixture park and exercise freshly rendered B. This
avoids relying on unrelated mouse events to schedule B's new native frame.

Strengthen retained-input proof beyond the owner's dismissal guard: retain A's
actual motion Rc, dispatch its old native press/move before redraw, assert no
native pointer capture before mouse-up, and require even A's retained motion
to stay at its old open position. Thus replacing only the closed callback guard
cannot make the geometry-admission fixture pass. New B remains uninitialized
until its own redraw and must still work afterward. No product test hook added.
