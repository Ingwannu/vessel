# Sidebar vertical layout

## Resume point

The repair starts from native integration `1d1f11df`. The theme shortcut
candidate is kept on a separate branch and its queued gate is not cancelled.
The control-only producer now supplies a shared viewport and real GPUI
geometry, wheel and Tab-focus probes. The four focused kit tests pass,
including wheel-away with retained focus and Shift+Tab return. The first
checks exposed a builder order error and a test borrow error; both were
corrected. Narrow clippy then found three missing test closure semicolons,
corrected in this checkpoint. Narrow lint confirmation is still pending. The
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
These differences do not justify extra scrolling regions or an unbounded body.

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
