# Shared ghost button

[Decision Log]
- Purpose: load-error retry and other quiet actions use Paseo's ghost button.
- Existing asset/constraint: `button_with_kind` already owns filled controls,
  shared geometry and typed enabled/disabled state; the bool wrapper stays.
- Alternative: a per-screen transparent override or local stand-in would drift
  in hover, pressed, disabled or text values.
- Choice: `ButtonKind::Ghost`, transparent fill/border and muted text, changing
  only the text to foreground on hover. Press changes opacity to 85%, disabled
  to 50%; disabled callers attach no handler, as before.
- Why: the same entry owns all variants, without changing an existing caller.
  Only ghost installs a hover override; filled buttons retain their idle paint.
- Cost: one kind and no dependency or token. Consumers migrate as their owners
  touch them. No app caller is included in this commit.

Reference read whole: Paseo `components/ui/button.tsx`, `styles/theme.ts` and
`components/ui/control-geometry.ts`. No source or asset was copied.

| Value | Light | Default dark |
|---|---|---|
| Fill and border | transparent | transparent |
| Idle/pressed/disabled text | `#71717a` | `#a1a5a4` |
| Hovered text | `#1a1a1e` | `#fafafa` |
| Pressed opacity | `0.85` | `0.85` |
| Disabled opacity | `0.5` | `0.5` |

Hover changes only text, press only opacity: a pointer press keeps the hovered
foreground. Touch press keeps the idle muted text. Geometry and label wrapping
remain shared with the existing kinds. Exact-value tests cover all four kinds
and idle/hover/pressed/disabled paint in both themes, rather than comparing a
variant to a token that could drift with it. The old bool API still selects only
Primary or Secondary and delegates unchanged. No focus treatment is added.
