# The host-name edit button

## Decision Log

- Purpose: a single shared host-name edit control for machine details.
- Constraint: Paseo host-appearance-section.tsx HostRenameButton has no density wrapper or hover/pressed palette.
- Alternatives: MD Ghost geometry, or an area-owned second pencil button.
- Choice: host_rename_button(id, ButtonState, tokens), original existing Pencil14, padding4/radius6, static muted ink, visual22 and outside8 giving hit38 in both layouts.
- Why: those are the source values. This38-point touch target is a source-specific exception, distinct from the ordinary44-point MD button.
- Impact: callers own callbacks and pending state; disabled consumers attach no callback. The control refuses its cursor/handler when disabled and adds no opacity, spinner or paint absent from the source.

Tests check the static geometry, actual SVG ink and returned hit frame with no opacity in both themes. Native evidence must cover outside/inside input, disabled state, adjacent controls and parent clipping. This control does not own a rename editor.

## After V2's review (row 65)

- The values come from tokens: the glyph is `theme::icon::SM` (Paseo's
  `ICON_SIZE.sm`), the radius `theme::radius::MD` (Paseo's
  `borderRadius.md`), the padding `spacing::S1`, the slop `spacing::S2`.
  The 22 frame and the 38 hit are computed from them, so a padding or slop
  change moves both. The test compares the ink with the muted token, not
  with a restated colour.
- Difference from Paseo, stated: Paseo's pencil is a button named for
  assistive technology (role button, the edit label). The kit has no
  accessibility tree and no focusable buttons yet, so this control has no
  accessible name and no keyboard reach. It takes no label until the kit
  can carry one, and shows no tooltip, since Paseo's pencil shows none.
- Moving the kit's other 14 and 6 values (`menu::draw::LEADING_GLYPH`,
  `popup::ROW_RADIUS`, `copy.rs`, `output.rs`) onto the two tokens is left
  to the kit's own cleanup.
