# Typed placement and timing in the existing tooltip controller

Status: source candidate, not compiled or executed. Existing composer and other
`Tooltips::on` / `on_code` callers retain immediate/top/eight-point behavior.
Toolbar uses the new profile only through its same shared hitbox/controller.

## Decision Log

- Purpose and intent: match the pane toolbar's desktop hover delay and placement
  without repainting the composer's intentionally immediate top tooltip.
- Existing assets and constraints: `kit::tooltip::Tooltips` already owns one
  screen's tooltip, trigger bounds and overlay. Its text/code faces, colors,
  margins and animation remain shared. The toolbar leaf documented timing/side
  as an unresolved difference; ordinary buttons are a separate shared control.
- Alternatives: change all defaults, add an area timer/tooltip overlay, or add
  typed configuration to that same controller.
- Choice: `Side::{Top,Bottom}`, checked finite nonnegative `Offset`, typed
  `Delay(Duration)` and `Configuration` with preserved defaults. The toolbar
  chooses bottom/eight/300ms; its explicit card-refresh top variant changes only
  side. Both use the existing text/render path. `ToolbarButton::tooltip_text`
  lets the owner supply resolved status/freshness words while the existing
  accessibility label continues to name the action; it adds no content formatter.
- Why: timing and placement are shared-control source facts; area-owned hover
  controllers would duplicate cancellation and disagree across surfaces.
- Benefits, costs and effects: one owned cancellable pending task at most,
  never detached. It weakly references the controller and exact installed trigger
  listener; original Place identity is refreshed on same-trigger rebuild. An
  unmounted listener cannot be kept alive by its delayed tip. Trigger ABA uses
  private Rc task identity rather than another counter or map. Existing default
  callers allocate no hover timer. No new crate/dependency is added.

## Pinned reference

Paseo 0.15.4, commit `1c00b365c9e0b74c1580eba33d905d693125ae21`, paths
relative to `packages/app/src`:

- `components/ui/pane-content-toolbar.tsx:88-159`: default side bottom; content
  centered at offset eight; action and menu both use delay 300, desktop enabled,
  mobile disabled. Existing typed menu/operation admission remains caller-owned.
- `components/ui/tooltip.tsx`: the shared tooltip implementation owns show/hide,
  side placement and timer cleanup. The native controller continues its existing
  card renderer; it does not create a separate Toolbar overlay.
- `usage/card.tsx:229-254`: card refresh explicitly uses a top freshness tooltip;
  `usage/refresh-button.tsx:20-33` and `usage/options.tsx:41-58` use the default.

The existing composer's immediate/top profile is deliberate and remains the
default. This change does not claim keyboard/focus/a11y or complete source fade
parity, a header-island profile, menu behavior, loading glyph/animation, or actual
Usage/Header consumption. Geometry and opacity of ordinary Button are untouched.

## Lifetime and verification

Pending state owns its GPUI Task; task drop cancels in the pinned scheduler.
Leave, press and a new trigger clear the existing pending task. The async closure
retains only weak controller/trigger references and a real originating Window
handle. Delivery checks the same private pending identity and trigger lifetime;
closed Window update refuses rather than looking up another window. The moved tip
becomes the same controller's sole shown value; there is no second overlay state.

Same-Place redraw updates pending AND shown words, side, offset and trigger
lifetime synchronously; GPUI does not emit another hover-enter for a stationary
pointer. A delay prop change does not restart an already scheduled hover timer:
the original enter deadline stays fixed, and the new delay applies on the next
enter. This follows pinned `tooltip.tsx:316-340`: scheduleOpen captures its delay,
and the cleanup effect does not depend on delay changes. Content/position still
reflect the current render, before the pending deadline and after it is shown.

Authored literal cases cover both anchors/offsets, original default, exact 300ms,
invalid offsets, and explicit top variant. Real GPUI fixtures exercise immediate
legacy hover, delayed toolbar hover, A/B/A, press cancellation, unmounted trigger,
controller drop, compact suppression and both pending/shown stationary redraw
using the deterministic test clock.
They have not run; delayed delivery/unmount behavior remains unverified until
actual checks, review and the lead's consumer union gate. No runtime claim is
borrowed from the earlier Toolbar geometry tests.
