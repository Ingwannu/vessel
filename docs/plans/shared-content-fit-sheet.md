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
