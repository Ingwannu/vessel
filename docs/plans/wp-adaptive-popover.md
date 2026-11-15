# Shared adaptive content popover

## Existing assets and decision

Modify existing assets: GPUI anchored/deferred placement and kit::popup already
provide a floating surface. The kit dialog owns compact sheets, typed layout,
snap gestures and G4's weak ReturnFocus. The action-menu renderer cannot carry
arbitrary settings controls without changing their input semantics. No dependency
or second sheet renderer is introduced.

[Decision Log]
- Purpose: W5 UsageOptions needs a 340px end-aligned desktop form and compact sheet.
- Constraints: preserve one dialog/snap renderer and current actual-close focus policy.
- Alternatives: area popup, menu-row encoding, or a new external UI framework.
- Choice: kit::popover composes anchored popup primitives and configured dialog.
- Reason: the same retained focus owner serves both presentations; fields remain caller-owned.
- Tradeoff: callers retain domain context and admit dismissal; the control's opaque lease independently rejects stale rendered callbacks.

## Reference

Paseo `packages/app/src/usage/options.tsx:38-90` uses compactMode=sheet,
DropdownMenuContent align=end, width340 and an arbitrary settings body.
`components/ui/menu/menu-surface.tsx:213-285` supplies offset4, horizontal
viewport padding16 for full-width positioning, an anchored popover without a
header. This is not body padding: MenuPage at menu-item.tsx:384-387 adds only
vertical4; settings rows own their horizontal16. Its compact branch at354-425
uses the existing adaptive sheet, title, natural content sizing and an80% cap.
The native popover adopts that cap and caller-owned row spacing; explicit
CompactLayout may select the same shared draggable snap engine. The shared
dialog's existing header/close styling remains the native compact header.

## API and data flow

kit::popover::PopoverState::open(bounds, Alignment, window, cx) returns an opaque
Lease. The feature stores that lease beside its captured ConnectionContext or
other owner context. Width reuses the validated positive DialogWidth type.
PopoverContent carries an ElementId, compact title, arbitrary AnyElement body,
width and existing CompactLayout. popover renders None while closed.

PopoverHandlers supplies state access plus admission returning Dismissal::Close
or KeepOpen. Escape, outside, compact header and snap closure use one captured
lease. A stale lease is rejected before the domain callback; admission is checked
before state/focus mutation, and close rechecks after admission in case it opened
a newer surface. Only actual close acknowledges DialogState::dismissed. Callers
must not implement another return-focus path. Replacing an opening finishes its
old focus/snap cycle, preventing retained old gestures from changing new motion.

Desktop is content-sized, viewport-capped, end/start aligned, offset4, popup
surface/radius/shadow and QUICK motion. Compact calls the existing dialog with the
same DialogState; no duplicate compact layout or gesture engine exists. This is
a generic body control; host readiness, preferences and feature actions stay out.
DialogLayout::with_body_spacing(BodySpacing::Caller) prevents double-insetting
forms; the legacy Inset24 default and existing dialog tests stay unchanged.

## Validation and handoff

Tests use actual TextField bodies, measured desktop340/end alignment, compact
dialog bounds, Escape/outside/header focus return, pending refusal, retained A-to-B
rendered callbacks and newer-focus preservation. Formatting and file budgets are
local source checks only. Compilation, Clippy and test execution run remotely.
No runtime, V1 or consumer-ready claim exists until their exact results land.

The control producer is isolated on the reviewed dd13 package line with only the
typed dialog/snap prerequisite and this control. V1 reviews it, the lead lands it,
then W5 consumes that checked producer rather than the desktop shell WIP branch.
