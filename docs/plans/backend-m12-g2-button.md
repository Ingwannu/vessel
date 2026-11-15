# Shared filled button kinds

[Decision Log]
- Purpose: use Paseo's destructive button in confirmations, once in the kit.
- Existing asset/constraint: `widgets::button` has the right pill geometry but
  accepts only a primary bool; existing callers must retain their signature.
- Alternatives: per-screen colour overrides, or migrate every caller now.
- Choice: `ButtonKind::{Primary, Secondary, Destructive}` and
  `ButtonState::{Enabled, Disabled}` at `button_with_kind`. The old `button`
  delegates with Enabled and is explicitly left for the lead to retire.
- Why: all filled kinds share geometry, wrapping, hover/press/disabled rules;
  the destructive foreground is independent of the brand's accent foreground.
- Cost: one neutral token is added for `destructiveForeground`. As with the
  existing outline control, disabled callers attach no click handler.

Sources read whole: Paseo `components/ui/button.tsx`; its destructive/default/
secondary styles and `styles/theme.ts` light and default-dark builders. No code
or assets were copied. Primary maps Paseo's `default` to Vessel's brand accent.
Outline/ghost controls are outside this change.

| Value | Light | Default dark |
|---|---|---|
| Destructive fill and border | `#b04138` | `#c64f43` |
| Destructive text | `#ffffff` | `#ffffff` |
| Hover | unchanged fill, border and text | unchanged fill, border and text |
| Pressed opacity | `0.85` | `0.85` |
| Disabled opacity | `0.5` | `0.5` |

Base geometry stays the kit's: minimum height 32, padding 16 across, gap 8,
fully rounded, Label type and a wrapping centred label. Touch callers keep
raising the minimum height to 44. No focus treatment was added. The new
entry has no pointer/hover/pressed treatment when disabled.

Exact-value tests cover every kind and phase in both themes, plus a changed
accent foreground to prevent accidentally reusing it for destructive text.
The README lists the API. This commit changes no app caller; the schedule
confirmation swaps its one call only after coordinator acceptance.


Review follow-up: the redundant hover palette assignment was removed; Paseo
sets no hover override on a filled button. The exact-value test names the
hover case and checks the idle paint that pointer entry retains.
