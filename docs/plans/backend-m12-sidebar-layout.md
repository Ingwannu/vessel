# Sidebar vertical layout

## Resume point

The repair starts from native integration `1d1f11df`. The theme shortcut
candidate is kept on a separate branch and its queued gate is not cancelled.
The control-only producer now supplies a shared viewport and real GPUI
geometry, wheel and Tab-focus probes. The four focused kit tests pass,
including wheel-away with retained focus and Shift+Tab return. The first
checks exposed a builder order error and a test borrow error; both were
corrected. Narrow clippy then found three missing test closure semicolons,
corrected in this checkpoint. Narrow clippy passes after those corrections. The control-first source
precheck found no API blocker, allowing the consumer to be installed; this
is not a final package or sidebar visual pass. The
sidebar consumes it only in a subsequent change, after control source review.
Under the package gate policy, each commit receives narrow checks; the whole
native gate and capture submission cover the completed layout package once.

## Reference and defect

Paseo 0.15.4 `components/left-sidebar.tsx:561–609,733–794` keeps the
navigation/header and footer outside `SidebarWorkspaceList`. Its mobile
content has `flex: 1` and `minHeight: 0`; the footer is not part of the list.
`components/sidebar-workspace-list.tsx:1941–2066,2479–2591` renders project,
pinned and workspace sections inside one `ScrollView` (native uses the
nestable variant). Nested draggable lists have `scrollEnabled={false}`.
`components/sidebar/sidebar-status-list.tsx:255–295,1080–1093` uses the same
one-body arrangement for grouped sessions. Both list styles use available
flex space, horizontal padding 8, top padding 2 and bottom padding 16.
There is no separate project/session height ratio or cap.

The current native sidebar instead fixes the projects above a scrollable
session region. Eight project rows alone exceed the available body height at
800×600. Expanding eleven projects also removes the session list and settings
foot at 1280×800. Moving the wheel cannot recover content outside that region.
The width evidence and its visual hold remain valid until the repair has new
captures; a passing compiler gate does not close the hold.

## Shared control contract (proposed)

[Decision Log]
- Purpose: keep fixed siblings in view while arbitrarily tall, already
  composed sections share the remaining vertical space and one scroll offset.
- Existing assets: `kit::VirtualList` owns virtualized, indexed row rendering
  and log reveal. Dialog and autocomplete privately use GPUI `ScrollHandle`
  and scrolling divs. There is no general composed-content viewport. The
  coordinator reserved this missing control before implementation.
- Alternatives: another area-local scroller would duplicate the behavior.
  Flattening both area-owned renderers into virtual row delegates would
  change their ownership and rendering contracts just to fix this layout.
  A new scrolling dependency or physics engine is unnecessary.
- Choice: a thin `ScrollViewport` / `ScrollState` over GPUI's own handle.
  The viewport fills available flex space with a zero minimum height; its
  sections keep their natural heights. State exposes measured viewport and
  section bounds and the offset. Wheel input remains GPUI's. A small shared
  focus target reveals its measured bounds when it gains focus.
- Why: this preserves the existing section renderers while giving the body
  the same single scrolling boundary as Paseo. Fixed header/footer siblings
  cannot be displaced by section content.
- Effects: no application types in the kit, no dependency or wire changes,
  and no duplicated renderer. Dialog and autocomplete are possible later
  consumers; their internals are deliberately not migrated in this slice.
  Existing buttons have broader keyboard/accessibility gaps; the minimal
  focus-reveal target does not claim full row activation or accessibility.

## Consumer structure

One body renderer contains Projects, the list heading, pinned rows and the
machine/status session groups. Navigation/header, Add machine and the footer
stay outside it with fixed natural heights. The old nested session scroller
is removed. No per-section height budget is introduced. Existing kit controls
and token spacing remain the source of the drawn rows.

Native differences retained with reasons: the project and session sections
remain separate parts of one body (their existing area ownership); Add
machine stays a fixed native navigation action beside the footer; the native
foot keeps its existing build/reload/Settings content. Paseo's list nesting
supports drag gestures, while this repair introduces no drag implementation.
The existing native outer inset (8) and section gap (12), expressed through
kit tokens, are retained to preserve the reviewed row rails. Paseo's list
content uses top 2 and bottom 16; this repair does not claim new inset or
complete sidebar pixel parity. The change is the shared bounded scrolling
boundary, with no extra scrolling region or per-section height allocation.

## Verification

- Shared GPUI control tests measure fixed siblings, viewport, section heights
  and scroll offsets at 800×600 and 1280×800 with folded/unfolded project
  shapes. Wheel input reaches the last item; keyboard focus reveals an
  offscreen target without moving the footer. No sleeps or widened deadlines.
- Consumer tests exercise the actual shared viewport placement and retain
  natural section heights and one body offset when the projects expand.
- Capture both window sizes, both themes, English and Korean, folded and
  unfolded, including the scrolled end. Open every original; record the
  source executable, dimensions and hashes. Keep failing evidence separate.
- Control-only source/gate goes to review first; consumer source and captures
  follow separately, using the full remote native gate for changed code.
  No layout pass, real input-method proof or full row accessibility is implied
  by a geometry/focus probe.

## Consumer checkpoint

The sidebar now keeps one persistent shared scroll state. Its three measured
sections contain Projects, the list heading and the existing machine/status
group renderer. Navigation, Add machine and the settings foot have fixed
natural heights. A minimal focus target wraps the actual last session or
"N more" row; the existing New conversation row and Settings button provide
the other two focus checkpoints. No invisible end row is added. No activation
semantics or complete row keyboard accessibility is claimed.

The test-only isolated shell constructor is shared within the app crate so
these real sidebar tests reuse its no-I/O setup. The tests exercise actual
project/session renderers in both grouping modes, expanding eleven projects,
wheel travel and Tab reach to the final row and fixed Settings button. This
consumer has passed its focused GPUI test in both grouping modes, at both
window sizes. Narrow clippy also passes. The capture executable was built
from the same product code; the package gate and final visual review remain
owed until the evidence is complete.

## Verification checkpoint

The shared viewport's four focused tests pass, and the actual sidebar's
focused test passes across two sizes and both grouping modes. They measure
natural sections, fixed siblings and the offset, exercise wheel reach and
fold clamping, and dispatch actual GPUI Tab/Shift+Tab navigation. Retaining
focus does not pull a subsequent wheel back to the focused item.

The initial capture matrix covers both sizes, both themes and languages,
with additional status-group and stored-width-600 cases. All 64 original
frames were opened. In the inspected cases Add machine, reload and Settings
stay visible at the same position, and the wheel reaches the final existing
row. Unfolding eleven projects does not displace the footer. Some wide
frames have an incidental composer tooltip outside the sidebar; these are
not a full-app visual approval. Long status-group titles at width 200 keep
the previous heavy-ellipsis limitation. Additional middle-position frames
make the unfolded project tail and the fold action directly visible.

No permission policy or service account changes are made. The local build,
headless probes and isolated fake fleet run as the normal developer account.
Full row keyboard activation/accessibility, a phone-layout visual pass and
platform-native screen-reader behavior are outside this repair's evidence.
