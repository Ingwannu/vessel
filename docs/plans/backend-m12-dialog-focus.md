# Return keyboard focus after an actual dialog close

[Decision Log]
- Purpose: Escape, scrim, close glyph and footer actions return input to the
  opener without stealing focus from a newer field or modal.
- Existing asset: one shared dialog renderer and GPUI weak focus handles.
  Unit close callbacks cannot report whether pending/stale policy refused.
  Menu and combobox have equivalent local policies, but no shared primitive.
- Alternatives: restore after every callback, hold a strong opener, capture
  a new opener on repeated focus, or add another focus manager/renderer.
- Choice: one `kit::focus::ReturnFocus` stores an explicit unopened/captured
  weak cycle, including captured absence. Controls remember before taking
  focus and acknowledge actual close through restore-if-owned.
- Why: the feature remains the authority over dismissal; repeated focus and
  refused dismissal cannot replace or consume its original opener. A later
  focus owner is preserved, while an actual close always consumes its cycle.
- Costs: existing unrelated consumers stay unadopted until they acknowledge
  actual close. There is no inference from a unit callback or global manager.

## API and behavior

`ReturnFocus::remember(&Window, &App)` captures once per open cycle.
`restore_if_owned(&FocusHandle, &mut Window, &mut App)` takes the cycle,
restores its live weak opener only if the control or a child still owns
focus, otherwise preserves current focus. A dead/missing opener blurs the
old owner. Captured absence differs from unopened, and never gets replaced
by a later repeated focus. No strong opener is retained.

`DialogState::focus` delegates to remember before taking keyboard input.
`DialogState::dismissed` delegates to restore-if-owned. The feature calls
that method only after its actual-close decision; pending/stale refusals
must not call it. All four close paths use the same acknowledgement. The
renderer, function-item/closure API, geometry, animation and focus ring
policy remain unchanged. Menu/select/combobox consume the primitive from
their own controls; this producer changes none of those renderers.

Paseo's `components/adaptive-modal-sheet.tsx:639–722` funnels Escape and
its close callback through one overlay and shared platform modal. GPUI
requires explicit return ownership instead of assuming that platform
behavior. This is an input-lifetime adaptation, not a new modal style.

## Verification and limits

The producer regression activates all four actual rendered close paths,
refuses pending and old-decision closes, preserves a newer focus, proves
reopen captures a fresh cycle, drops a deleted opener, and distinguishes
captured absence. Narrow verification is pending; no runtime or UI pass
is claimed. The whole feature package gate and consuming Devices stale
context/intent proof remain separate. Unadopted callers do not receive
implicit restoration, and the generic helper makes no domain decision.


The first exact-source remote narrow Clippy run on `fe53a206` stopped on
two test-only semicolon-format lints; no test ran and no pass is claimed
from that run. The correction changes those statements only. The failed
run remains retained as `devices-narrow-20261115T080025Z-fe53a206`.


A source follow-up also refuses to restore an opener that is the owner or
one of its children. Such a handle can be focused before a late open hook;
returning to it would leave input in the dismissed subtree. The regression
models an already-focused owner, then opens/closes through the public API;
focus must be cleared. This changes no signature or geometry.
