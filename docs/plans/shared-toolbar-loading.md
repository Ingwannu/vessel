# Shared toolbar loading content

Status: source candidate only. No compile/runtime/consumer handoff approval.
The independent typed tooltip profile leaf remains separately scoped.

## Decision Log

- Purpose and intent: use the actual source activity indicator inside the SAME
  toolbar chrome while refresh is busy; a static Spinner SVG is not that source.
- Existing assets and constraints: `kit::loading::activity_indicator` already
  renders the source track/arc masks and 750ms motion. Toolbar owns source20/32
  frames and fixed/adaptive glyph geometry; ordinary Button is a different API.
- Alternatives: an area spinner/size/ink override, duplicate ActivityIndicator
  rendering, or reuse that renderer with canonical numeric14/18 points.
- Choice: add only `Points::{FOURTEEN,EIGHTEEN}` and a named
  `ToolbarButton::loading` constructor. Both normal/loading constructors use one
  private static chrome renderer. Loading forces disabled state, including its
  existing half opacity and handler refusal, preserving selection/disclosure.
- Why: a busy owner can reuse source geometry and animation without another
  control, a changed ordinary button or a late click handler bypass.
- Benefits, costs and effects: no vector, motion or dependency changes. G2's
  concurrent unpublished FOURTEEN addition has the same meaning and must converge
  to this single definition. Existing points/pixels/TEN/SIXTEEN stay unchanged.
  The layout callback still owns request/context/tickets; a widget is not effect
  admission. Ready rendering resumes only after the owner changes its state.

## Pinned source and verification limits

Paseo 0.15.4 commit `1c00b365c9e0b74c1580eba33d905d693125ae21`, relative
to `packages/app/src`: `usage/refresh-button.tsx:20-33` and
`usage/card.tsx:229-254` choose disabled busy controls, extra-muted icon mapping
and `LoadingSpinner` with `paneContentToolbarIconSize` numeric14/18.
`components/ui/loading-spinner.tsx` delegates to ActivityIndicator; the existing
native masks/750ms profile remain its one implementation. `usage/options.tsx`
uses the same chrome around its Gear menu action, without a loading state.

Authored literal and real GPUI cases cover numeric points, extra-muted ink,
disabled opacity, identical frame bounds across Busy/Ready and actual no-click
while busy even if a caller supplied an enabled configuration. Both cases now
pass in the exact receiving union recorded below.
The existing geometry/input assertions remain. Actual Usage data, header-island
round chrome, menu behavior, keyboard/focus/a11y and rendered native/OS parity are
outside this source candidate; no test or visual result is borrowed from G2/F1.

## Executed receiving union

Exact `b9df6b605c37539544732878974758faada7a611` completed strict run
`wp-layout-core-union-20261116T003646Z-b9df6b60` and same-head runtime
`wp-layout-core-union-20261116T005309Z-b9df6b60`, fonts/cargo0 for both.
Runtime compiled View/Kit/Link/App and passed1527/0/2 ignored in total, including
the explicit numeric-points/ink/disabled-opacity and Busy/Ready no-click cases.
All four owned loading Rust blobs equal that executed source; Toolbar also
contains the selected Tooltip methods. The selected portable native tree equals
the executed union. This is receiving integration evidence, not a new-commit or
isolated-leaf runtime claim, native-quality/capture, Usage or Header approval.
