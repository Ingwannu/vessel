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

Automatic owner retirement is separate from user-dismiss admission. When a host
ConnectionContext retires or the area closes, take its captured context/Lease once
so the feature stops drawing the obsolete body immediately. A settle hook with
only &Window can use the existing GPUI Context::defer_in(&Window, closure) to obtain
the mutable Window at the admitted event boundary. That closure calls
state.close(&retired_lease, window, cx) and notifies, without another generation or
state-only close. If a new opening replaced it meanwhile, close rejects the old
lease before any focus/snap acknowledgment. The owner handles new-route focus;
G4's shared policy preserves focus already taken by that route. No mutation is
performed during drawing. This follows vendor app/context.rs:301-313.

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
Initial isolated2a all-target Kit Clippy reached the source and reported one test
callback type-inference error, a similar-name test binding, missing doc backticks
and viewport arithmetic lints. The corrected source uses explicit callback type,
distinct names and fused finite layout arithmetic, with no allowances. Tests have
not run yet. No runtime, V1 or consumer-ready claim exists until exact results land.
The second exact1a Clippy typechecked the control/tests and reported only two
unary float-negation lints on the margin constant. Current source uses a named
signed viewport inset; no lint allowance is added. The next base8 source check
must also execute the real control tests before review.

The control producer is isolated with only the typed dialog/snap prerequisite and
this control. It began on dd13; package base8/91407173 is now merged in0313d40a9
before its next remote run, including the reviewed S2 menu lease and FieldId.
The queued Clippy2 job still inspects its selected1a/dd13 source, not this union.
V1 reviews the checked producer, the lead lands it,
then W5 consumes that checked producer rather than the desktop shell WIP branch.
